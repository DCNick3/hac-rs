use crate::storage::{ReadableStorage, Storage, StorageError};

#[derive(Debug, Clone)]
pub enum EitherStorage<L: ReadableStorage, R: ReadableStorage> {
    Left(L),
    Right(R),
}

impl<L: ReadableStorage, R: ReadableStorage> ReadableStorage for EitherStorage<L, R> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        match self {
            EitherStorage::Left(storage) => storage.read(offset, buf),
            EitherStorage::Right(storage) => storage.read(offset, buf),
        }
    }

    fn get_size(&self) -> u64 {
        match self {
            EitherStorage::Left(storage) => storage.get_size(),
            EitherStorage::Right(storage) => storage.get_size(),
        }
    }
}

impl<L: Storage, R: Storage> Storage for EitherStorage<L, R> {
    fn write(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError> {
        match self {
            EitherStorage::Left(storage) => storage.write(offset, buf),
            EitherStorage::Right(storage) => storage.write(offset, buf),
        }
    }

    fn flush(&self) -> Result<(), StorageError> {
        match self {
            EitherStorage::Left(storage) => storage.flush(),
            EitherStorage::Right(storage) => storage.flush(),
        }
    }

    fn set_size(&self, new_size: u64) -> Result<(), StorageError> {
        match self {
            EitherStorage::Left(storage) => storage.set_size(new_size),
            EitherStorage::Right(storage) => storage.set_size(new_size),
        }
    }
}
