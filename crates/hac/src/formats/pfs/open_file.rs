use crate::formats::pfs::PartitionFileSystem;
use snafu::{ResultExt, Snafu};

#[derive(Snafu, Debug)]
pub enum PfsOpenFileError {
    StorageError {
        source: crate::storage::StorageError,
    },
    PfsParseError {
        source: crate::formats::pfs::PfsParseError,
    },
}

impl PartitionFileSystem<crate::storage::FileRoStorage> {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, PfsOpenFileError> {
        let storage = crate::storage::FileRoStorage::open(path).context(StorageSnafu)?;
        Self::new(storage).context(PfsParseSnafu)
    }
}
