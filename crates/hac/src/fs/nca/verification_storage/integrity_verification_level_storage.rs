use crate::fs::nca::verification_storage::{
    IntegrityCheckLevel, IntegrityStorageType, DIGEST_SIZE,
};
use crate::fs::storage::{
    ReadableBlockStorage, ReadableBlockStorageExt, ReadableStorage, StorageError,
};
use digest::Digest;
use num::Integer;
use sha2::Sha256;
use std::ops::{Deref, DerefMut};
use std::slice::SliceIndex;
use std::sync::Mutex;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum BlockStatus {
    Unchecked,
    Invalid,
    Valid,
}

#[derive(Debug)]
pub struct IntegrityVerificationLevelStorage<S: ReadableBlockStorage, H: ReadableStorage> {
    storage: S,
    hash_storage: H,
    level: IntegrityCheckLevel,
    ty: IntegrityStorageType,
    block_statuses: Mutex<Vec<BlockStatus>>,
}

impl<S: ReadableBlockStorage, H: ReadableStorage> IntegrityVerificationLevelStorage<S, H> {
    pub fn new(
        storage: S,
        hash_storage: H,
        level: IntegrityCheckLevel,
        ty: IntegrityStorageType,
    ) -> Self {
        let block_count = Integer::div_ceil(&storage.get_size(), &storage.block_size());
        let block_statuses = vec![BlockStatus::Unchecked; block_count.try_into().unwrap()];

        Self {
            storage,
            hash_storage,
            level,
            ty,
            block_statuses: Mutex::new(block_statuses),
        }
    }
}

enum BlockBuffer<'a> {
    Borrowed(&'a mut [u8]),
    Owned(Vec<u8>),
}

impl Deref for BlockBuffer<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            BlockBuffer::Borrowed(buf) => buf,
            BlockBuffer::Owned(buf) => buf,
        }
    }
}

impl DerefMut for BlockBuffer<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            BlockBuffer::Borrowed(buf) => buf,
            BlockBuffer::Owned(buf) => buf,
        }
    }
}

impl<I: SliceIndex<[u8]>> std::ops::Index<I> for BlockBuffer<'_> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        match self {
            BlockBuffer::Borrowed(buf) => &buf[index],
            BlockBuffer::Owned(buf) => &buf[index],
        }
    }
}

impl<I: SliceIndex<[u8]>> std::ops::IndexMut<I> for BlockBuffer<'_> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        match self {
            BlockBuffer::Borrowed(buf) => &mut buf[index],
            BlockBuffer::Owned(buf) => &mut buf[index],
        }
    }
}

impl<S: ReadableBlockStorage, H: ReadableStorage> ReadableBlockStorage
    for IntegrityVerificationLevelStorage<S, H>
{
    fn block_size(&self) -> u64 {
        self.storage.block_size()
    }

    fn read_block(&self, block_index: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        let block_size = self.storage.block_size();

        // handle the trailing block, which may be smaller than the block size
        let current_block_size = self.nth_block_size(block_index);

        // handle partial block reads
        let mut block_buf = if buf.len() as u64 == block_size {
            BlockBuffer::Borrowed(buf)
        } else {
            BlockBuffer::Owned(vec![0; block_size as usize])
        };

        self.storage
            .read_block(block_index, &mut block_buf[..current_block_size as usize])?;

        if self.level == IntegrityCheckLevel::None {
            if let BlockBuffer::Owned(block_buf) = block_buf {
                buf.copy_from_slice(&block_buf[..buf.len()]);
            }
            return Ok(());
        }

        let mut block_statuses = self.block_statuses.lock().unwrap();
        let block_status = &mut block_statuses[block_index as usize];

        if *block_status == BlockStatus::Unchecked {
            let bytes_to_hash = match self.ty {
                IntegrityStorageType::PartitionFs => {
                    // PartitionFs does not pad the last block
                    current_block_size
                }
                IntegrityStorageType::RomFs => {
                    // pad the unused part of the buffer (handling the last block, which may be smaller than the block size)
                    block_buf[current_block_size as usize..].fill(0);
                    block_size
                }
            };

            let hash = Sha256::digest(&block_buf[..bytes_to_hash as usize]);
            let mut expected_hash = [0; DIGEST_SIZE];
            self.hash_storage
                .read(block_index * DIGEST_SIZE as u64, &mut expected_hash)?;

            *block_status = if hash.as_slice() == expected_hash {
                BlockStatus::Valid
            } else {
                BlockStatus::Invalid
            };
        }

        if *block_status == BlockStatus::Invalid && self.level == IntegrityCheckLevel::Full {
            return Err(StorageError::IntegrityCheckFailed {});
        }

        if let BlockBuffer::Owned(block_buf) = block_buf {
            buf.copy_from_slice(&block_buf[..buf.len()]);
        }
        Ok(())
    }

    fn get_size(&self) -> u64 {
        self.storage.get_size()
    }
}
