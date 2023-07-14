mod streaming_zstd_storage;

use crate::hexstring::HexData;
use crate::storage::{
    BlockAdapterStorage, BlockCacheStorage, ConcatStorageN, LinearAdapterStorage, ReadableStorage,
    ReadableStorageExt, SharedStorage, SliceStorage, StorageError, StorageIo,
};
use streaming_zstd_storage::StreamingZstdStorage;

use binrw::{BinRead, BinReaderExt, BinWrite};
use itertools::Either;
use snafu::{ResultExt, Snafu};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::time::Duration;

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

const NCZ_MAGIC: &[u8; 8] = b"NCZSECTN";
const NCZ_BLOCK_MAGIC: &[u8; 8] = b"NCZBLOCK";

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

const NCA_HEADERS_SIZE: u64 = 0x4000;

#[derive(Debug)]
pub enum NczBodyStorage<S: ReadableStorage> {
    Streaming(
        LinearAdapterStorage<
            BlockCacheStorage<BlockAdapterStorage<StreamingZstdStorage<SliceStorage<S>>>>,
        >,
    ),
    Block(
        LinearAdapterStorage<
            BlockCacheStorage<
                BlockAdapterStorage<
                    ConcatStorageN<StreamingZstdStorage<SliceStorage<SharedStorage<S>>>>,
                >,
            >,
        >,
    ),
}

fn make_cache<S: ReadableStorage>(
    storage: S,
    block_size: u64,
    cache_blocks: u64,
    time_to_idle: Duration,
) -> LinearAdapterStorage<BlockCacheStorage<BlockAdapterStorage<S>>> {
    let storage = BlockAdapterStorage::new(storage, block_size);
    let storage = BlockCacheStorage::new(storage, cache_blocks, time_to_idle);
    let storage = LinearAdapterStorage::new(storage);

    storage
}

impl<S: ReadableStorage> ReadableStorage for NczBodyStorage<S> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        // add a fake unreadable region at the beginning of the storage
        // this makes sure the section offsets add up
        if offset < NCA_HEADERS_SIZE {
            return Err(StorageError::Inaccessible { offset });
        }
        let offset = offset - NCA_HEADERS_SIZE;

        match self {
            Self::Streaming(storage) => storage.read(offset, buf),
            Self::Block(storage) => storage.read(offset, buf),
        }
    }

    fn get_size(&self) -> u64 {
        (match self {
            Self::Streaming(storage) => storage.get_size(),
            Self::Block(storage) => storage.get_size(),
        }) + NCA_HEADERS_SIZE
    }
}

impl<S: ReadableStorage> NczBodyStorage<S> {
    fn make_block(
        mut reader: BufReader<StorageIo<S>>,
        _header: NczHeader,
        total_size: u64,
    ) -> Result<NczBodyStorage<S>, NczError> {
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

        Ok(NczBodyStorage::Block(make_cache(
            uncompressed_storage,
            1024 * 1024,
            64,
            Duration::from_millis(500),
        )))
    }

    fn make_stream(
        mut reader: BufReader<StorageIo<S>>,
        header: NczHeader,
        total_size: u64,
    ) -> Result<NczBodyStorage<S>, NczError> {
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
        let uncompressed_storage = StreamingZstdStorage::new(compressed_storage, uncompressed_size)
            .context(StorageSnafu)?;

        Ok(NczBodyStorage::Streaming(make_cache(
            uncompressed_storage,
            512 * 1024,
            128,
            Duration::from_millis(500),
        )))
    }

    /// Checks whether the file is an NCZ and returns storage for uncompressed body
    ///
    /// The header is unavailable in this storage
    pub fn try_new(storage: S) -> Result<Either<NczBodyStorage<S>, S>, NczError> {
        let total_size = storage.get_size();

        let mut ncz_magic = [0; 8];
        storage
            .read(NCA_HEADERS_SIZE, &mut ncz_magic)
            .context(StorageSnafu)?;
        if &ncz_magic != NCZ_MAGIC {
            return Ok(Either::Right(storage));
        }

        let mut reader = storage.buf_read();
        reader
            .seek(SeekFrom::Start(NCA_HEADERS_SIZE))
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

        if &block_magic == NCZ_BLOCK_MAGIC {
            Self::make_block(reader, header, total_size)
        } else {
            Self::make_stream(reader, header, total_size)
        }
        .map(Either::Left)
    }
}
