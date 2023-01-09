use snafu::ResultExt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Mutex;

use super::{IoSnafu, ReadableStorage, Storage, StorageError};

pub struct RoIoStorage<Io: Read + Seek> {
    io: Mutex<Io>,
    size: u64,
}

impl<Io: Read + Seek> RoIoStorage<Io> {
    pub fn new(mut io: Io) -> Result<Self, StorageError> {
        let size = io
            .seek(SeekFrom::End(0))
            .context(IoSnafu { operation: "seek" })?;
        io.seek(SeekFrom::Start(0))
            .context(IoSnafu { operation: "seek" })?;
        Ok(Self {
            io: Mutex::new(io),
            size,
        })
    }

    fn check_size<'a>(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError> {
        let end = offset + buf.len() as u64;
        if end > self.size {
            Err(StorageError::OutOfBounds {})
        } else {
            Ok(())
        }
    }
}

impl RwIoStorage<File> {
    pub fn new_file(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let io = File::open(path).context(IoSnafu { operation: "open" })?;
        Self::new(io)
    }
}

impl<Io: Read + Seek> ReadableStorage for RoIoStorage<Io> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        self.check_size(offset, buf)?;
        let mut io = self.io.lock().unwrap();
        io.seek(SeekFrom::Start(offset))
            .context(IoSnafu { operation: "seek" })?;
        io.read_exact(buf).context(IoSnafu { operation: "read" })?;
        Ok(())
    }

    fn get_size(&self) -> u64 {
        self.size
    }
}

impl<Io: Read + Seek> Storage for RoIoStorage<Io> {
    fn write(&self, _offset: u64, _buf: &[u8]) -> Result<(), StorageError> {
        Err(StorageError::Readonly {})
    }

    fn flush(&self) -> Result<(), StorageError> {
        Err(StorageError::Readonly {})
    }

    fn set_size(&self, _new_size: u64) -> Result<(), StorageError> {
        Err(StorageError::Readonly {})
    }
}

struct RwIoStorageInner<Io: Read + Write + Seek> {
    io: Io,
    size: u64,
}

impl<Io: Read + Write + Seek> RwIoStorageInner<Io> {
    fn check_size(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError> {
        let end = offset + buf.len() as u64;
        if end > self.size {
            Err(StorageError::OutOfBounds {})
        } else {
            Ok(())
        }
    }
}

/// A storage that wraps an IO object, allowing read and write access.
///
/// Note that this storage does not implement resizing correctly, as there is no trait for that =(.
pub struct RwIoStorage<Io: Read + Write + Seek>(Mutex<RwIoStorageInner<Io>>);

impl<Io: Read + Write + Seek> RwIoStorage<Io> {
    pub fn new(mut io: Io) -> Result<Self, StorageError> {
        let size = io
            .seek(SeekFrom::End(0))
            .context(IoSnafu { operation: "seek" })?;
        io.seek(SeekFrom::Start(0))
            .context(IoSnafu { operation: "seek" })?;
        Ok(Self(Mutex::new(RwIoStorageInner { io, size })))
    }
}

impl<Io: Read + Write + Seek> ReadableStorage for RwIoStorage<Io> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        let mut inner = self.0.lock().unwrap();
        inner.check_size(offset, buf)?;
        inner
            .io
            .seek(SeekFrom::Start(offset))
            .context(IoSnafu { operation: "seek" })?;
        inner.io.read_exact(buf).context(IoSnafu {
            operation: "read_exact",
        })?;
        Ok(())
    }

    fn get_size(&self) -> u64 {
        let mut inner = self.0.lock().unwrap();
        inner.size
    }
}

impl<Io: Read + Write + Seek> Storage for RwIoStorage<Io> {
    fn write(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError> {
        let mut inner = self.0.lock().unwrap();
        inner.check_size(offset, buf)?;
        inner
            .io
            .seek(SeekFrom::Start(offset))
            .context(IoSnafu { operation: "seek" })?;
        inner.io.write_all(buf).context(IoSnafu {
            operation: "write_all",
        })?;
        Ok(())
    }

    fn flush(&self) -> Result<(), StorageError> {
        let mut inner = self.0.lock().unwrap();
        inner.io.flush().context(IoSnafu { operation: "flush" })?;
        Ok(())
    }

    fn set_size(&self, new_size: u64) -> Result<(), StorageError> {
        let mut inner = self.0.lock().unwrap();
        // at least try to expand the file
        // no way to shrink it, though =(
        inner
            .io
            .seek(SeekFrom::Start(new_size))
            .context(IoSnafu { operation: "seek" })?;
        inner.size = new_size;
        Ok(())
    }
}
