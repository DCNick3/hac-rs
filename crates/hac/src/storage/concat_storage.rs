use crate::storage::{ReadableStorage, StorageError};

pub struct ConcatStorage2<Left, Right> {
    left: Left,
    right: Right,
}

impl<Left: ReadableStorage, Right: ReadableStorage> ConcatStorage2<Left, Right> {
    pub fn new(left: Left, right: Right) -> Self {
        Self { left, right }
    }
}

impl<Left: ReadableStorage, Right: ReadableStorage> ReadableStorage
    for ConcatStorage2<Left, Right>
{
    fn read(&self, mut offset: u64, mut buf: &mut [u8]) -> Result<(), StorageError> {
        let left_size = self.left.get_size();

        if offset < left_size {
            let end = std::cmp::min(offset + buf.len() as u64, left_size);
            let len = (end - offset) as usize;

            self.left.read(offset, &mut buf[..len])?;

            offset += len as u64;
            buf = &mut buf[len..];
        }

        if !buf.is_empty() {
            self.right.read(offset - left_size, buf)?;
        }

        Ok(())
    }

    fn get_size(&self) -> u64 {
        self.left.get_size() + self.right.get_size()
    }
}

pub struct ConcatStorageN<S> {
    storages: Vec<S>,
}

impl<S: ReadableStorage> ConcatStorageN<S> {
    pub fn new(storages: Vec<S>) -> Self {
        Self { storages }
    }
}

impl<S: ReadableStorage> ReadableStorage for ConcatStorageN<S> {
    fn read(&self, mut offset: u64, mut buf: &mut [u8]) -> Result<(), StorageError> {
        for storage in &self.storages {
            let size = storage.get_size();

            if offset < size {
                let end = std::cmp::min(offset + buf.len() as u64, size);
                let len = (end - offset) as usize;

                storage.read(offset, &mut buf[..len])?;

                offset += len as u64;
                buf = &mut buf[len..];
            }

            if buf.is_empty() {
                break;
            }
        }

        Ok(())
    }

    fn get_size(&self) -> u64 {
        self.storages.iter().map(|s| s.get_size()).sum()
    }
}
