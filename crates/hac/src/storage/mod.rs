use num_integer::Integer;
use snafu::Snafu;
use std::io::BufReader;
use std::path::Path;

mod block_adapter_storage;
mod block_slice_storage;
mod block_transform_storage;
mod either_storage;
mod io_storage;
mod linear_adapter_storage;
mod shared_storage;
mod slice_storage;
mod storage_io;
mod vec_storage;

pub use block_adapter_storage::BlockAdapterStorage;
pub use block_slice_storage::{BlockSliceStorage, BlockSliceStorageError};
pub use block_transform_storage::{
    block_transforms, AesCtrStorage, BlockTransform, BlockTransformStorage,
};
pub use either_storage::EitherStorage;
pub use io_storage::{FileRoStorage, FileRwStorage, RoIoStorage, RwIoStorage};
pub use linear_adapter_storage::LinearAdapterStorage;
pub use shared_storage::SharedStorage;
pub use slice_storage::{SliceStorage, SliceStorageError};
pub use storage_io::StorageIo;
pub use vec_storage::VecStorage;

pub trait ReadableStorage: Send + Sync {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError>;
    fn get_size(&self) -> u64;
}

pub trait Storage: ReadableStorage {
    fn write(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError>;
    fn flush(&self) -> Result<(), StorageError>;
    fn set_size(&self, new_size: u64) -> Result<(), StorageError>;
}

pub trait ReadableBlockStorage: Send + Sync {
    fn block_size(&self) -> u64;
    /// Reads block at given index.
    ///
    /// Note: this allows reading partial blocks at the end of the storage.
    fn read_block(&self, block_index: u64, buf: &mut [u8]) -> Result<(), StorageError>;
    /// Gets size of storage in bytes.
    ///
    /// Note: this does NOT have to be a multiple of block size.
    fn get_size(&self) -> u64;

    fn read_block_bulk(&self, block_index: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        let block_size = self.block_size();
        let block_count = Integer::div_ceil(&(buf.len() as u64), &block_size);
        for i in 0..block_count {
            let buf = &mut buf[(i * block_size) as usize..];
            let read_size = std::cmp::min(block_size, buf.len() as u64);
            self.read_block(block_index + i as u64, &mut buf[..read_size as usize])?;
        }
        Ok(())
    }
}

pub trait BlockStorage: ReadableBlockStorage {
    fn write_block(&self, block_index: u64, buf: &[u8]) -> Result<(), StorageError>;
    fn flush(&self) -> Result<(), StorageError>;
    fn set_size(&self, new_size: u64) -> Result<(), StorageError>;

    fn write_block_bulk(&self, block_index: u64, buf: &[u8]) -> Result<(), StorageError> {
        let block_size = self.block_size();
        let block_count = Integer::div_ceil(&(buf.len() as u64), &block_size);
        for i in 0..block_count {
            let buf = &buf[(i * block_size) as usize..];
            let write_size = std::cmp::min(block_size, buf.len() as u64);
            self.write_block(block_index + i as u64, &buf[..write_size as usize])?;
        }
        Ok(())
    }
}

pub trait ReadableStorageExt: ReadableStorage {
    fn slice(self, offset: u64, size: u64) -> Result<SliceStorage<Self>, SliceStorageError>
    where
        Self: Sized,
    {
        SliceStorage::new(self, offset, size)
    }

    fn shared(self) -> SharedStorage<Self>
    where
        Self: Sized,
    {
        SharedStorage::new(self)
    }

    fn io(self) -> StorageIo<Self>
    where
        Self: Sized,
    {
        StorageIo::new(self)
    }

    fn buf_read(self) -> BufReader<StorageIo<Self>>
    where
        Self: Sized,
    {
        BufReader::new(self.io())
    }

    fn read_all(&self) -> Result<Vec<u8>, StorageError> {
        let mut buf = vec![0; self.get_size() as usize];
        self.read(0, &mut buf)?;
        Ok(buf)
    }

    fn copy_to<S: Storage>(&self, other: &S) -> Result<(), StorageError> {
        const BUFFER_SIZE: usize = 0x10000;
        let size = self.get_size();
        other.set_size(size)?;
        let mut buf = vec![0; BUFFER_SIZE];
        for offset in (0..size).step_by(BUFFER_SIZE) {
            let read_size = std::cmp::min(BUFFER_SIZE as u64, size - offset);
            self.read(offset, &mut buf[..read_size as usize])?;
            other.write(offset, &buf[..read_size as usize])?;
        }
        Ok(())
    }

    fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), StorageError> {
        self.copy_to(&FileRwStorage::create(path)?)
    }
}

pub trait ReadableBlockStorageExt: ReadableBlockStorage {
    fn slice(
        self,
        block_offset: u64,
        size: u64,
    ) -> Result<BlockSliceStorage<Self>, BlockSliceStorageError>
    where
        Self: Sized,
    {
        BlockSliceStorage::new(self, block_offset, size)
    }

    fn block_count(&self) -> u64 {
        Integer::div_ceil(&self.get_size(), &self.block_size())
    }

    fn nth_block_size(&self, block_index: u64) -> u64 {
        assert!(block_index < self.block_count());
        if block_index == self.block_count() - 1 {
            // the last block may be smaller than the block size
            // compute its size
            ((self.get_size() - 1) % self.block_size()) + 1
        } else {
            self.block_size()
        }
    }
}

impl<T: ReadableStorage> ReadableStorageExt for T {}
impl<T: ReadableBlockStorage> ReadableBlockStorageExt for T {}

#[derive(Snafu, Debug)]
pub enum StorageError {
    #[snafu(display("IO error in IoStorage: {}", source))]
    Io {
        source: std::io::Error,
        operation: &'static str,
    },
    #[snafu(display("Attempt to write to a read-only storage"))]
    Readonly {},
    #[snafu(display("Attempt to resize a fixed-size storage"))]
    FixedSize {},
    #[snafu(display("Attempt to read or write to a storage out of bounds"))]
    OutOfBounds {},
    #[snafu(display("Integrity check failed"))]
    IntegrityCheckFailed {},
    #[snafu(display("A storage requiring aligned access was accessed with an unaligned offset"))]
    UnalignedAccess {},
}
