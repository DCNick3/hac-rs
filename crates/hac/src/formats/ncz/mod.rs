mod streaming_zstd_storage;

use crate::formats::ncz::streaming_zstd_storage::StreamingZstdStorage;
use crate::hexstring::HexData;
use crate::storage::{ReadableStorage, ReadableStorageExt, StorageError};
use binrw::{BinRead, BinReaderExt, BinWrite};
use snafu::{ResultExt, Snafu};
use std::io::{Read, Seek};

#[derive(Snafu, Debug)]
pub enum NcaError {
    Storage { source: StorageError },
    NczHeaderParsing { source: binrw::Error },
    FsHeaderHashMismatch { index: usize },
    StorageSizeMismatch { expected: u64, actual: u64 },
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

const NCA_HEADERS_SIZE: usize = 0x4000;

pub struct NczStorage<S: ReadableStorage> {
    storage: S,
    nca_header: [u8; NCA_HEADERS_SIZE],
    header: NczHeader,
    nca_size: u64,
}

impl<S: ReadableStorage> NczStorage<S> {
    pub fn new(storage: S) -> Result<Self, NcaError> {
        let mut nca_header = [0; NCA_HEADERS_SIZE];
        storage.read(0, &mut nca_header).context(StorageSnafu)?;

        let total_size = storage.get_size();

        let mut reader = storage.buf_read();
        reader
            .seek(std::io::SeekFrom::Start(NCA_HEADERS_SIZE as u64))
            .unwrap();

        let header: NczHeader = reader.read_le().context(NczHeaderParsingSnafu)?;

        let compress_start = reader.stream_position().unwrap();
        let compress_size = total_size - compress_start;

        let mut block_magic = [0; 8];
        reader
            .read_exact(&mut block_magic)
            .map_err(binrw::Error::from)
            .context(NczHeaderParsingSnafu)?;

        if &block_magic == b"NCZBLOCK" {
            todo!("Block NCZs are not supported yet")
        }

        let storage = reader.into_inner().into_inner();

        let uncompressed_size = header
            .section_headers
            .iter()
            .map(|section| section.size)
            .sum();

        let compressed_storage = storage.slice(compress_start, compress_size).unwrap();
        let uncompressed_storage = StreamingZstdStorage::new(compressed_storage, uncompressed_size)
            .context(StorageSnafu)?;

        uncompressed_storage
            .save_to_file("test_files/ncz/stream/ed5f53408e88b7d2974e3b6cce8bfa57.nczz")
            .unwrap();

        // TODO: re-encrypt the NCA

        todo!()
    }
}
