pub mod block_transforms;

use crate::storage::block_transforms::AesCtrBlockTransform;
use crate::storage::{BlockStorage, ReadableBlockStorage, StorageError};

pub trait BlockTransform: Clone + Send + Sync {
    const BLOCK_SIZE: u64;

    /// Transform a block of data when reading from underlying storage.
    ///
    /// Allows to transform multiple blocks at once, for example to decrypt
    fn transform_read(&self, block: &mut [u8], block_index: u64);
    fn transform_write(&self, block: &mut [u8], block_index: u64);
}

#[derive(Debug, Clone)]
pub struct BlockTransformStorage<S: ReadableBlockStorage, T: BlockTransform> {
    storage: S,
    transform: T,
}

pub type AesCtrStorage<S> = BlockTransformStorage<S, AesCtrBlockTransform>;

impl<S: ReadableBlockStorage, T: BlockTransform> BlockTransformStorage<S, T> {
    pub fn new(storage: S, transform: T) -> Self {
        assert_eq!(
            storage.get_size() % T::BLOCK_SIZE as u64,
            0,
            "Storage size must be a multiple of the block size"
        );
        assert_eq!(
            storage.block_size(),
            T::BLOCK_SIZE,
            "Storage block size must match transform block size"
        );
        Self { storage, transform }
    }
}

impl<S: ReadableBlockStorage, T: BlockTransform> ReadableBlockStorage
    for BlockTransformStorage<S, T>
{
    fn block_size(&self) -> u64 {
        self.storage.block_size()
    }

    fn read_block(&self, block_index: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        assert_eq!(
            buf.len() as u64,
            T::BLOCK_SIZE,
            "Only full blocks can be read"
        );

        self.storage.read_block(block_index, buf)?;
        self.transform.transform_read(buf, block_index);

        Ok(())
    }

    fn get_size(&self) -> u64 {
        self.storage.get_size()
    }

    fn read_block_bulk(&self, block_index: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        assert_eq!(
            buf.len() as u64 % T::BLOCK_SIZE,
            0,
            "Only full blocks can be read"
        );

        self.storage.read_block_bulk(block_index, buf)?;

        // transform_read allows to transform multiple blocks at once
        self.transform.transform_read(buf, block_index);

        Ok(())
    }
}

impl<S: BlockStorage, T: BlockTransform> BlockStorage for BlockTransformStorage<S, T> {
    fn write_block(&self, _block_index: u64, _buf: &[u8]) -> Result<(), StorageError> {
        todo!()
    }

    fn flush(&self) -> Result<(), StorageError> {
        self.storage.flush()
    }

    fn set_size(&self, new_size: u64) -> Result<(), StorageError> {
        self.storage.set_size(new_size)
    }
}
