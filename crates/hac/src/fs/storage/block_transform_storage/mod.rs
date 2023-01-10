pub mod block_transforms;

use crate::fs::storage::block_transforms::AesCtrBlockTransform;
use crate::fs::storage::{ReadableStorage, Storage, StorageError};

pub trait BlockTransform: Clone + Send + Sync {
    const BLOCK_SIZE: usize;

    /// Transform a block of data when reading from underlying storage.
    ///
    /// Allows to transorm multiple blocks at once, for example to decrypt
    fn transform_read(&self, block: &mut [u8], block_index: u64);
    fn transform_write(&self, block: &mut [u8], block_index: u64);
}

#[derive(Debug, Clone)]
pub struct BlockTransformStorage<S: ReadableStorage, T: BlockTransform> {
    storage: S,
    transform: T,
}

pub type AesCtrStorage<S> = BlockTransformStorage<S, AesCtrBlockTransform>;

impl<S: ReadableStorage, T: BlockTransform> BlockTransformStorage<S, T> {
    pub fn new(storage: S, transform: T) -> Self {
        assert_eq!(
            storage.get_size() % T::BLOCK_SIZE as u64,
            0,
            "Storage size must be a multiple of the block size"
        );
        Self { storage, transform }
    }
}

impl<S: ReadableStorage, T: BlockTransform> ReadableStorage for BlockTransformStorage<S, T> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        // NOTE: this implementation is __not very efficient__ (buffer is too small)
        let mut tmp = vec![0; 1024 * 64];
        assert_eq!(tmp.len() % T::BLOCK_SIZE, 0);
        let block_offset = offset % T::BLOCK_SIZE as u64;
        assert_eq!(
            block_offset, 0,
            "Unaligned reads are not supported (at least for now)"
        );
        assert_eq!(
            buf.len() % T::BLOCK_SIZE,
            0,
            "Unaligned reads are not supported (at least for now)"
        );
        let mut buf_offset = 0;
        while buf_offset < buf.len() {
            let block_index = (offset + buf_offset as u64) / T::BLOCK_SIZE as u64;
            let read_size = std::cmp::min(tmp.len(), buf.len() - buf_offset);
            self.storage.read(
                (offset + buf_offset as u64) / T::BLOCK_SIZE as u64 * T::BLOCK_SIZE as u64,
                &mut tmp[..read_size],
            )?;
            self.transform
                .transform_read(&mut tmp[..read_size], block_index);
            let copy_size = std::cmp::min(read_size, buf.len() - buf_offset);
            buf[buf_offset..buf_offset + copy_size].copy_from_slice(&tmp[..copy_size]);
            buf_offset += copy_size;
        }
        Ok(())
    }

    fn get_size(&self) -> u64 {
        self.storage.get_size()
    }
}

impl<S: Storage, T: BlockTransform> Storage for BlockTransformStorage<S, T> {
    fn write(&self, _offset: u64, _buf: &[u8]) -> Result<(), StorageError> {
        todo!()

        // this impl is untested!!
        // NOTE: this implementation is __not very efficient__ (buffer is too small)
        // let mut block = vec![0; T::BLOCK_SIZE];
        // let mut block_offset = offset % T::BLOCK_SIZE as u64;
        // let mut buf_offset = 0;
        // while buf_offset < buf.len() {
        //     let write_size = std::cmp::min(
        //         T::BLOCK_SIZE - block_offset as usize,
        //         buf.len() - buf_offset,
        //     );
        //     if block_offset != 0 {
        //         self.storage
        //             .read(offset + buf_offset as u64 - block_offset, &mut block)?;
        //         self.transform.transform_read(&mut block);
        //     }
        //     block[block_offset as usize..block_offset as usize + write_size]
        //         .copy_from_slice(&buf[buf_offset..buf_offset + write_size]);
        //     self.transform.transform_write(&mut block, offset / T::BLOCK_SIZE as u64);
        //     self.storage
        //         .write(offset + buf_offset as u64 - block_offset, &block)?;
        //     buf_offset += write_size;
        //     block_offset = 0;
        // }
        // Ok(())
    }

    fn flush(&self) -> Result<(), StorageError> {
        self.storage.flush()
    }

    fn set_size(&self, new_size: u64) -> Result<(), StorageError> {
        self.storage.set_size(new_size)
    }
}
