use crate::storage::{BlockStorage, ReadableBlockStorage, ReadableStorage, Storage, StorageError};

#[derive(Debug)]
pub struct LinearAdapterStorage<S: ReadableBlockStorage> {
    storage: S,
}

impl<S: ReadableBlockStorage> LinearAdapterStorage<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

impl<S: ReadableBlockStorage> ReadableStorage for LinearAdapterStorage<S> {
    fn read(&self, mut offset: u64, mut buf: &mut [u8]) -> Result<(), StorageError> {
        // TODO: use small vec
        let mut block_buffer = vec![0u8; self.storage.block_size() as usize];

        // read head (block-unaligned start)
        let head_block_offset = offset % self.storage.block_size();
        if head_block_offset != 0 {
            let head_block_index = offset / self.storage.block_size();
            self.storage
                .read_block(head_block_index, &mut block_buffer)?;
            let head_block_size = self.storage.block_size() - head_block_offset;
            let head_block_size = std::cmp::min(head_block_size, buf.len() as u64);
            buf[..head_block_size as usize].copy_from_slice(
                &block_buffer[head_block_offset as usize..][..head_block_size as usize],
            );

            offset += head_block_size;
            buf = &mut buf[head_block_size as usize..];
        }

        // read body (block-aligned center)
        let body_block_count = buf.len() / self.storage.block_size() as usize;
        self.storage.read_block_bulk(
            offset / self.storage.block_size(),
            &mut buf[..body_block_count * self.storage.block_size() as usize],
        )?;

        offset += body_block_count as u64 * self.storage.block_size();
        buf = &mut buf[body_block_count * self.storage.block_size() as usize..];

        // read tail (block-unaligned end)
        if !buf.is_empty() {
            let tail_block_index = offset / self.storage.block_size();
            self.storage
                .read_block(tail_block_index, &mut block_buffer)?;
            buf.copy_from_slice(&block_buffer[..buf.len()]);
        }

        Ok(())
    }

    fn get_size(&self) -> u64 {
        self.storage.get_size()
    }
}

impl<S: BlockStorage> Storage for LinearAdapterStorage<S> {
    fn write(&self, _offset: u64, _buf: &[u8]) -> Result<(), StorageError> {
        // this is kinda nasty, requiring us to read unaligned blocks before writing
        todo!()
    }

    fn flush(&self) -> Result<(), StorageError> {
        self.storage.flush()
    }

    fn set_size(&self, new_size: u64) -> Result<(), StorageError> {
        self.storage.set_size(new_size)
    }
}
