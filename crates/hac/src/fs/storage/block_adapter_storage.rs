use crate::fs::storage::{
    BlockStorage, ReadableBlockStorage, ReadableStorage, Storage, StorageError,
};

#[derive(Debug)]
pub struct BlockAdapterStorage<S: ReadableStorage> {
    storage: S,
    block_size: u64,
}

impl<S: ReadableStorage> BlockAdapterStorage<S> {
    pub fn new(storage: S, block_size: u64) -> Self {
        Self {
            storage,
            block_size,
        }
    }
}

impl<S: ReadableStorage> ReadableBlockStorage for BlockAdapterStorage<S> {
    fn block_size(&self) -> u64 {
        self.block_size
    }

    fn read_block(&self, block_index: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        assert_eq!(
            buf.len() as u64,
            self.block_size,
            "Only full blocks can be read"
        );
        let offset = block_index * self.block_size;
        self.storage.read(offset, buf)
    }

    fn get_size(&self) -> u64 {
        self.storage.get_size()
    }

    fn read_block_bulk(&self, block_index: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        assert_eq!(
            buf.len() as u64 % self.block_size,
            0,
            "Only full blocks can be read"
        );
        let offset = block_index * self.block_size;
        self.storage.read(offset, buf)
    }
}

impl<S: Storage> BlockStorage for BlockAdapterStorage<S> {
    fn write_block(&self, block_index: u64, buf: &[u8]) -> Result<(), StorageError> {
        assert_eq!(
            buf.len() as u64,
            self.block_size,
            "Only full blocks can be written"
        );
        let offset = block_index * self.block_size;
        self.storage.write(offset, buf)
    }

    fn flush(&self) -> Result<(), StorageError> {
        self.storage.flush()
    }

    fn set_size(&self, new_size: u64) -> Result<(), StorageError> {
        self.storage.set_size(new_size)
    }

    fn write_block_bulk(&self, block_index: u64, buf: &[u8]) -> Result<(), StorageError> {
        assert_eq!(
            buf.len() as u64 % self.block_size,
            0,
            "Only full blocks can be written"
        );
        let offset = block_index * self.block_size;
        self.storage.write(offset, buf)
    }
}
