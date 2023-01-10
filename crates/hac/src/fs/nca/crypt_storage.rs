use crate::fs::storage::{AesCtrStorage, ReadableStorage, Storage, StorageError};

#[derive(Debug, Clone)]
pub enum NcaCryptStorage<S: ReadableStorage> {
    Plaintext(S),
    AesCtr(AesCtrStorage<S>),
}

impl<S: ReadableStorage> ReadableStorage for NcaCryptStorage<S> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        match self {
            NcaCryptStorage::Plaintext(storage) => storage.read(offset, buf),
            NcaCryptStorage::AesCtr(storage) => storage.read(offset, buf),
        }
    }

    fn get_size(&self) -> u64 {
        match self {
            NcaCryptStorage::Plaintext(storage) => storage.get_size(),
            NcaCryptStorage::AesCtr(storage) => storage.get_size(),
        }
    }
}

impl<S: Storage> Storage for NcaCryptStorage<S> {
    fn write(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError> {
        match self {
            NcaCryptStorage::Plaintext(storage) => storage.write(offset, buf),
            NcaCryptStorage::AesCtr(storage) => storage.write(offset, buf),
        }
    }

    fn flush(&self) -> Result<(), StorageError> {
        match self {
            NcaCryptStorage::Plaintext(storage) => storage.flush(),
            NcaCryptStorage::AesCtr(storage) => storage.flush(),
        }
    }

    fn set_size(&self, new_size: u64) -> Result<(), StorageError> {
        match self {
            NcaCryptStorage::Plaintext(storage) => storage.set_size(new_size),
            NcaCryptStorage::AesCtr(storage) => storage.set_size(new_size),
        }
    }
}
