use crate::storage::{ReadableBlockStorage, ReadableBlockStorageExt, StorageError};
use mini_moka::sync::{Cache, CacheBuilder};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

pub struct BlockCacheStorage<S> {
    storage: S,
    cache: Cache<u64, Arc<Vec<u8>>>,
}

impl<S: ReadableBlockStorage> BlockCacheStorage<S> {
    pub fn new(storage: S, blocks_in_cache: u64, time_to_idle: Duration) -> Self {
        let cache = CacheBuilder::new(blocks_in_cache)
            .time_to_idle(time_to_idle)
            .build();

        Self { storage, cache }
    }
}

impl<S: ReadableBlockStorage> ReadableBlockStorage for BlockCacheStorage<S> {
    fn block_size(&self) -> u64 {
        self.storage.block_size()
    }

    fn read_block(&self, block_index: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        let block_size = self.nth_block_size(block_index) as usize;
        assert!(block_size <= buf.len());
        let buf = &mut buf[..block_size];

        match self.cache.get(&block_index) {
            Some(content) => {
                buf.copy_from_slice(content.as_slice());
                Ok(())
            }
            None => {
                self.storage.read_block(block_index, buf)?;
                // allocating on every cache miss is a bit sad..
                let content = Arc::new(buf.to_vec());
                self.cache.insert(block_index, content);
                Ok(())
            }
        }
    }

    fn get_size(&self) -> u64 {
        self.storage.get_size()
    }
}

impl<S: fmt::Debug> fmt::Debug for BlockCacheStorage<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockCacheStorage")
            .field("storage", &self.storage)
            .finish()
    }
}
