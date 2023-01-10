use crate::fs::storage::{
    BlockStorage, ReadableBlockStorage, ReadableStorage, Storage, StorageError,
};
use snafu::Snafu;

pub struct BlockSliceStorage<S> {
    storage: S,
    block_offset: u64,
    size: u64,
}

#[derive(Snafu, Debug)]
pub enum BlockSliceStorageError {
    OffsetOutOfBounds { offset: u64, bounds: u64 },
    SizeOutOfBounds { offset: u64, size: u64, bounds: u64 },
}

impl<S: ReadableBlockStorage> BlockSliceStorage<S> {
    pub fn new(storage: S, block_offset: u64, size: u64) -> Result<Self, BlockSliceStorageError> {
        let block_size = storage.block_size();
        let bounds = storage.get_size();
        if block_offset * block_size > bounds {
            return Err(BlockSliceStorageError::OffsetOutOfBounds {
                offset: block_offset * block_size,
                bounds,
            });
        }
        if block_offset * block_size + size > bounds {
            return Err(BlockSliceStorageError::SizeOutOfBounds {
                offset: block_offset * block_size,
                size,
                bounds,
            });
        }

        Ok(Self {
            storage,
            block_offset,
            size,
        })
    }
}

impl<S: ReadableBlockStorage> ReadableBlockStorage for BlockSliceStorage<S> {
    fn block_size(&self) -> u64 {
        self.storage.block_size()
    }

    fn read_block(&self, block_index: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        assert_eq!(buf.len() as u64, self.block_size());
        if block_index * self.block_size() + buf.len() as u64 > self.size {
            return Err(StorageError::OutOfBounds {});
        }
        self.storage
            .read_block(self.block_offset + block_index, buf)
    }

    fn get_size(&self) -> u64 {
        self.size
    }

    fn read_block_bulk(&self, block_index: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        assert_eq!(buf.len() as u64 % self.block_size(), 0);
        if block_index * self.block_size() + buf.len() as u64 > self.size {
            return Err(StorageError::OutOfBounds {});
        }
        self.storage
            .read_block_bulk(self.block_offset + block_index, buf)
    }
}

impl<S: BlockStorage> BlockStorage for BlockSliceStorage<S> {
    fn write_block(&self, block_index: u64, buf: &[u8]) -> Result<(), StorageError> {
        assert_eq!(buf.len() as u64, self.block_size());
        if block_index * self.block_size() + buf.len() as u64 > self.size {
            return Err(StorageError::OutOfBounds {});
        }
        self.storage
            .write_block(self.block_offset + block_index, buf)
    }

    fn flush(&self) -> Result<(), StorageError> {
        self.storage.flush()
    }

    fn set_size(&self, _new_size: u64) -> Result<(), StorageError> {
        Err(StorageError::FixedSize {})
    }

    fn write_block_bulk(&self, block_index: u64, buf: &[u8]) -> Result<(), StorageError> {
        assert_eq!(buf.len() as u64 % self.block_size(), 0);
        if block_index * self.block_size() + buf.len() as u64 > self.size {
            return Err(StorageError::OutOfBounds {});
        }
        self.storage
            .write_block_bulk(self.block_offset + block_index, buf)
    }
}
