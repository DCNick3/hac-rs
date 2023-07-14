use crate::storage::{ReadableStorage, StorageError};

#[derive(Debug)]
struct DenyStorage {
    size: u64,
}

impl ReadableStorage for DenyStorage {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        if offset + buf.len() as u64 > self.size {
            return Err(StorageError::OutOfBounds {});
        }

        Err(StorageError::Inaccessible { offset })
    }

    fn get_size(&self) -> u64 {
        self.size
    }
}
