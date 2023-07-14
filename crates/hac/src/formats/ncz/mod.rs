mod streaming_zstd_storage;

use crate::formats::ncz::streaming_zstd_storage::StreamingZstdStorage;
use crate::hexstring::HexData;
use crate::storage::{
    ConcatStorageN, ReadableStorage, ReadableStorageExt, SharedStorage, SliceStorage, StorageError,
};
use binrw::{BinRead, BinReaderExt, BinWrite};
use snafu::{ResultExt, Snafu};
use std::io::{Read, Seek, SeekFrom};

const BLOCK_EXPONENT_MIN: u8 = 14;
const BLOCK_EXPONENT_MAX: u8 = 32;

#[derive(Snafu, Debug)]
pub enum NczError {
    /// NCZ: Failed to read from the storage
    Storage { source: StorageError },
    /// Failed to parse the NCZ header
    NczHeaderParsing { source: binrw::Error },
    /// Invalid NCZ block size exponent: {exponent}, must be between {BLOCK_EXPONENT_MIN} and {BLOCK_EXPONENT_MAX}
    InvalidBlockSizeExponent { exponent: u8 },
    /// NCZ's size is not the same as the storage's size: expected {expected}, got {actual}
    SizeMismatch { expected: u64, actual: u64 },
}

#[derive(Debug, Clone, BinRead, BinWrite)]
struct NczSectionHeader {
    offset: u64,
    size: u64,
    #[br(pad_after = 0x8)]
    crypto_type: u64,

    crypto_key: HexData<0x10>,
    crypto_counter: HexData<0x10>,
}

#[derive(Debug, Clone, BinRead, BinWrite)]
#[br(magic = b"NCZSECTN")]
struct NczHeader {
    section_count: u64,
    #[br(count = section_count)]
    section_headers: Vec<NczSectionHeader>,
}

#[derive(Debug, Clone, BinRead, BinWrite)]
#[br(magic = b"NCZBLOCK")]
struct NczBlockHeader {
    #[br(assert(version == 0x2))]
    version: u8,
    ty: u8,
    #[brw(pad_before = 0x1)] // unused field
    block_size_exponent: u8,
    number_of_blocks: u32,
    total_decompressed_size: u64,
    #[br(count = number_of_blocks)]
    compressed_block_sizes: Vec<u32>,
}

const NCA_HEADERS_SIZE: usize = 0x4000;

pub struct Ncz<S: ReadableStorage> {
    storage: S,
    nca_header: [u8; NCA_HEADERS_SIZE],
    header: NczHeader,
    nca_size: u64,
}

enum NczBodyStorage<S: ReadableStorage> {
    Streaming(StreamingZstdStorage<SliceStorage<S>>),
    Block(ConcatStorageN<StreamingZstdStorage<SliceStorage<SharedStorage<S>>>>),
}

impl<S: ReadableStorage> ReadableStorage for NczBodyStorage<S> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        match self {
            Self::Streaming(storage) => storage.read(offset, buf),
            Self::Block(storage) => storage.read(offset, buf),
        }
    }

    fn get_size(&self) -> u64 {
        match self {
            Self::Streaming(storage) => storage.get_size(),
            Self::Block(storage) => storage.get_size(),
        }
    }
}

impl<S: ReadableStorage> Ncz<S> {
    pub fn new(storage: S) -> Result<Self, NczError> {
        let mut nca_header = [0; NCA_HEADERS_SIZE];
        storage.read(0, &mut nca_header).context(StorageSnafu)?;

        let total_size = storage.get_size();

        let mut reader = storage.buf_read();
        reader
            .seek(SeekFrom::Start(NCA_HEADERS_SIZE as u64))
            .expect("BUG: Failed to seek to NCZ header");

        let header: NczHeader = reader.read_le().context(NczHeaderParsingSnafu)?;

        let mut block_magic = [0; 8];
        reader
            .read_exact(&mut block_magic)
            .map_err(binrw::Error::from)
            .context(NczHeaderParsingSnafu)?;
        reader
            .seek(SeekFrom::Current(-(block_magic.len() as i64)))
            .expect("BUG: Failed to seek back to NCZ block header");

        let uncompressed_storage = if &block_magic == b"NCZBLOCK" {
            let block_header: NczBlockHeader = reader.read_le().context(NczHeaderParsingSnafu)?;

            let mut position = reader.stream_position().unwrap();

            if block_header.block_size_exponent < BLOCK_EXPONENT_MIN
                || block_header.block_size_exponent > BLOCK_EXPONENT_MAX
            {
                return Err(NczError::InvalidBlockSizeExponent {
                    exponent: block_header.block_size_exponent,
                });
            }

            let block_decompressed_size = 1u64 << block_header.block_size_exponent;

            let total_compressed_size = block_header
                .compressed_block_sizes
                .iter()
                .map(|&v| v as u64)
                .sum::<u64>();

            if position + total_compressed_size != total_size {
                return Err(NczError::SizeMismatch {
                    expected: position + total_compressed_size,
                    actual: total_size,
                });
            }

            let storage = reader.into_inner().into_inner().shared();

            let mut left_decompressed_size = block_header.total_decompressed_size;

            let mut block_storages = Vec::new();
            for &block_size in &block_header.compressed_block_sizes {
                let block_compressed_size = block_size as u64;
                let block_compressed_storage = storage
                    .clone()
                    .slice(position, block_compressed_size)
                    .expect("BUG: Failed to slice NCZ block");

                let block_decompressed_size =
                    std::cmp::min(block_decompressed_size, left_decompressed_size);

                if block_compressed_size == block_decompressed_size {
                    todo!("Handle uncompressed blocks")
                }

                let block_decompressed_storage =
                    StreamingZstdStorage::new(block_compressed_storage, block_decompressed_size)
                        .context(StorageSnafu)
                        .unwrap();

                position += block_compressed_size;
                left_decompressed_size -= block_decompressed_size;

                block_storages.push(block_decompressed_storage);
            }

            let uncompressed_storage = ConcatStorageN::new(block_storages);

            NczBodyStorage::Block(uncompressed_storage)
        } else {
            let compress_start = reader.stream_position().unwrap();
            let compress_size = total_size - compress_start;

            let storage = reader.into_inner().into_inner();

            let uncompressed_size = header
                .section_headers
                .iter()
                .map(|section| section.size)
                .sum();

            let compressed_storage = storage
                .slice(compress_start, compress_size)
                .expect("BUG: Failed to slice NCZ compressed storage");
            let uncompressed_storage =
                StreamingZstdStorage::new(compressed_storage, uncompressed_size)
                    .context(StorageSnafu)?;

            NczBodyStorage::Streaming(uncompressed_storage)
        };

        uncompressed_storage
            .save_to_file("test_files/ncz/block/ed5f53408e88b7d2974e3b6cce8bfa57.nczz")
            .unwrap();

        // TODO: re-encrypt the NCA

        todo!()
    }
}
