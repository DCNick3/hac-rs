use crate::storage::{ReadableStorage, Storage, StorageError};
use snafu::Snafu;

#[derive(Debug)]
pub struct SliceStorage<S> {
    storage: S,
    offset: u64,
    size: u64,
}

#[derive(Snafu, Debug)]
pub enum SliceStorageError {
    OffsetOutOfBounds { offset: u64, bounds: u64 },
    SizeOutOfBounds { offset: u64, size: u64, bounds: u64 },
}

impl<S: ReadableStorage> SliceStorage<S> {
    pub fn new(storage: S, offset: u64, size: u64) -> Result<Self, SliceStorageError> {
        let bounds = storage.get_size();
        if offset > bounds {
            return Err(SliceStorageError::OffsetOutOfBounds { offset, bounds });
        }
        if offset + size > bounds {
            return Err(SliceStorageError::SizeOutOfBounds {
                offset,
                size,
                bounds,
            });
        }

        Ok(Self {
            storage,
            offset,
            size,
        })
    }
}

impl<S: ReadableStorage> ReadableStorage for SliceStorage<S> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        if offset + buf.len() as u64 > self.size {
            return Err(StorageError::OutOfBounds {});
        }
        self.storage.read(self.offset + offset, buf)
    }

    fn get_size(&self) -> u64 {
        self.size
    }
}

impl<S: Storage> Storage for SliceStorage<S> {
    fn write(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError> {
        if offset + buf.len() as u64 > self.size {
            return Err(StorageError::OutOfBounds {});
        }
        self.storage.write(self.offset + offset, buf)
    }

    fn flush(&self) -> Result<(), StorageError> {
        self.storage.flush()
    }

    fn set_size(&self, _new_size: u64) -> Result<(), StorageError> {
        Err(StorageError::FixedSize {})
    }
}
