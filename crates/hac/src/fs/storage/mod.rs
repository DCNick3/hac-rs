use snafu::Snafu;
use std::path::Path;

mod io_storage;
mod slice_storage;

use crate::fs::storage::slice_storage::SliceStorageError;
pub use io_storage::{FileRoStorage, FileRwStorage, RoIoStorage, RwIoStorage};
pub use slice_storage::SliceStorage;

// I am not sure if it is a good idea to make ReadableStorage Clonable
// It allows all sorts of weird bugs (especially with regards to writing)
// but it is very convenient, for example to get access to NCA sections
pub trait ReadableStorage: Clone + Send + Sync {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError>;
    fn get_size(&self) -> u64;
}

pub trait Storage: ReadableStorage {
    fn write(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError>;
    fn flush(&self) -> Result<(), StorageError>;
    fn set_size(&self, new_size: u64) -> Result<(), StorageError>;
}

pub trait ReadableStorageExt: ReadableStorage {
    fn slice(self, offset: u64, size: u64) -> Result<SliceStorage<Self>, SliceStorageError>
    where
        Self: Sized,
    {
        SliceStorage::new(self, offset, size)
    }

    fn copy_to<S: Storage>(&self, other: &S) -> Result<(), StorageError> {
        const BUFFER_SIZE: usize = 0x10000;
        let size = self.get_size();
        other.set_size(size)?;
        let mut buf = vec![0; BUFFER_SIZE];
        for offset in (0..size).step_by(BUFFER_SIZE) {
            let read_size = std::cmp::min(BUFFER_SIZE as u64, size - offset);
            self.read(offset, &mut buf[..read_size as usize])?;
            other.write(offset, &buf[..read_size as usize])?;
        }
        Ok(())
    }

    fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), StorageError> {
        self.copy_to(&FileRwStorage::create(path)?)
    }
}

impl<T: ReadableStorage> ReadableStorageExt for T {}

#[derive(Snafu, Debug)]
pub enum StorageError {
    #[snafu(display("IO error in IoStorage: {}", source))]
    Io {
        source: std::io::Error,
        operation: &'static str,
    },
    #[snafu(display("Attempt to write to a read-only storage"))]
    Readonly {},
    #[snafu(display("Attempt to resize a fixed-size storage"))]
    FixedSize {},
    #[snafu(display("Attempt to read or write to a storage out of bounds"))]
    OutOfBounds {},
}
