use crate::storage::{ReadableStorage, Storage, StorageError};
use std::fmt::Debug;
use std::sync::RwLock;

pub struct VecStorage {
    data: RwLock<Vec<u8>>,
}

impl Debug for VecStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VecStorage").finish()
    }
}

impl VecStorage {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: RwLock::new(data),
        }
    }
}

impl ReadableStorage for VecStorage {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        let data = self.data.read().unwrap();

        let offset = offset.try_into().unwrap();
        let len = buf.len();
        buf.copy_from_slice(&data[offset..offset + len]);
        Ok(())
    }

    fn get_size(&self) -> u64 {
        let data = self.data.read().unwrap();

        data.len().try_into().unwrap()
    }
}

impl Storage for VecStorage {
    fn write(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError> {
        let mut data = self.data.write().unwrap();

        let offset = offset.try_into().unwrap();
        let len = buf.len();
        data[offset..offset + len].copy_from_slice(buf);
        Ok(())
    }

    fn flush(&self) -> Result<(), StorageError> {
        Ok(())
    }

    fn set_size(&self, new_size: u64) -> Result<(), StorageError> {
        let mut data = self.data.write().unwrap();

        let new_size = new_size.try_into().unwrap();
        data.resize(new_size, 0);
        Ok(())
    }
}
