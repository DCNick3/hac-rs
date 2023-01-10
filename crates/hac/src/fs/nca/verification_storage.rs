use crate::fs::storage::{ReadableStorage, StorageError};
use digest::Digest;
use num::Integer;
use sha2::Sha256;
use std::sync::Mutex;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum BlockStatus {
    Unchecked,
    Invalid,
    Valid,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IntegrityStorageType {
    PartitionFs,
    RomFs,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IntegrityCheckLevel {
    /// No integrity checks will be performed.
    None,
    /// Invalid blocks will be marked as invalid when read, and will not cause an error.
    IgnoreOnInvalid,
    /// An error will be returned when an invalid block is read.
    Full,
}

#[derive(Debug)]
struct IntegrityVerificationLevelStorageInner<S: ReadableStorage, H: ReadableStorage> {
    storage: S,
    hash_storage: H,
    block_size: u64,
    block_statuses: Vec<BlockStatus>,
    level: IntegrityCheckLevel,
    ty: IntegrityStorageType,
}

#[derive(Debug)]
pub struct IntegrityVerificationLevelStorage<S: ReadableStorage, H: ReadableStorage> {
    inner: Mutex<IntegrityVerificationLevelStorageInner<S, H>>,
}

const DIGEST_SIZE: usize = 0x20;

impl<S: ReadableStorage, H: ReadableStorage> IntegrityVerificationLevelStorage<S, H> {
    pub fn new(
        storage: S,
        hash_storage: H,
        block_size: u64,
        level: IntegrityCheckLevel,
        ty: IntegrityStorageType,
    ) -> Self {
        let block_count = Integer::div_ceil(&storage.get_size(), &block_size);
        let block_statuses = vec![BlockStatus::Unchecked; block_count.try_into().unwrap()];

        Self {
            inner: Mutex::new(IntegrityVerificationLevelStorageInner {
                storage,
                hash_storage,
                block_size,
                block_statuses,
                level,
                ty,
            }),
        }
    }
}

impl<S: ReadableStorage, H: ReadableStorage> ReadableStorage
    for IntegrityVerificationLevelStorage<S, H>
{
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        let mut inner = self.inner.lock().unwrap();

        let block_size = inner.block_size;

        if offset % block_size != 0 {
            return Err(StorageError::UnalignedAccess {});
        }

        let block_index = offset / block_size;

        let needs_hash_check = inner.level != IntegrityCheckLevel::None
            && inner.block_statuses[block_index as usize] == BlockStatus::Unchecked;

        if !needs_hash_check {
            return inner.storage.read(offset, buf);
        }

        let mut hash = [0; DIGEST_SIZE];
        inner
            .hash_storage
            .read(block_index * DIGEST_SIZE as u64, &mut hash)?;

        todo!()
    }

    fn get_size(&self) -> u64 {
        self.inner.lock().unwrap().storage.get_size()
    }
}
