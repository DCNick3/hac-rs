use crate::storage::{ReadableStorage, Storage, StorageError};
use std::io::{ErrorKind, Read};

pub struct StorageIo<S: ReadableStorage> {
    storage: S,
    position: u64,
}

impl<S: ReadableStorage> StorageIo<S> {
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            position: 0,
        }
    }

    pub fn into_inner(self) -> S {
        self.storage
    }
}

fn map_storage_error_to_std(error: StorageError) -> std::io::Error {
    let kind = match &error {
        StorageError::Io { source, .. } => source.kind(),
        StorageError::Inaccessible { .. } => ErrorKind::Other,
        StorageError::Readonly { .. } => ErrorKind::Other,
        StorageError::FixedSize { .. } => ErrorKind::Other,
        StorageError::OutOfBounds { .. } => ErrorKind::UnexpectedEof,
        StorageError::IntegrityCheckFailed { .. } => ErrorKind::InvalidData,
        StorageError::UnalignedAccess { .. } => ErrorKind::InvalidInput,
    };
    std::io::Error::new(kind, error)
}

impl<S: ReadableStorage> Read for StorageIo<S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = std::cmp::min(buf.len() as u64, self.storage.get_size() - self.position);
        self.storage
            .read(self.position, &mut buf[..read as usize])
            .map_err(map_storage_error_to_std)?;
        self.position += read;
        Ok(read as usize)
    }
}

impl<S: ReadableStorage> std::io::Seek for StorageIo<S> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let new_position = match pos {
            std::io::SeekFrom::Start(offset) => offset.try_into().unwrap(),
            std::io::SeekFrom::End(offset) => self.storage.get_size() as i64 + offset,
            std::io::SeekFrom::Current(offset) => self.position as i64 + offset,
        };
        if new_position < 0 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "Attempt to seek before the beginning of the storage",
            ));
        }
        if new_position > self.storage.get_size() as i64 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "Attempt to seek after the end of the storage",
            ));
        }
        self.position = new_position as u64;
        Ok(self.position)
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.position)
    }
}

impl<S: Storage> std::io::Write for StorageIo<S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.storage
            .write(self.position, buf)
            .map_err(map_storage_error_to_std)?;
        self.position += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.storage.flush().map_err(map_storage_error_to_std)
    }
}
