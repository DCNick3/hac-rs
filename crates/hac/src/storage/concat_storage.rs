use crate::storage::{ReadableStorage, StorageError};

#[derive(Debug)]
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

#[derive(Debug)]
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

            offset -= size;
        }

        Ok(())
    }

    fn get_size(&self) -> u64 {
        self.storages.iter().map(|s| s.get_size()).sum()
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::{ConcatStorageN, ReadableStorage, VecStorage};

    fn check_read<S: ReadableStorage>(storage: &S, offset: u64, expected: &[u8]) {
        let mut buf = vec![0; expected.len()];
        storage.read(offset, &mut buf).unwrap();
        assert_eq!(
            std::str::from_utf8(&buf).unwrap(),
            std::str::from_utf8(expected).unwrap()
        );
    }

    #[test]
    fn concat_n() {
        let storage = ConcatStorageN::new(vec![
            VecStorage::new(b"".to_vec()),
            VecStorage::new(b"1".to_vec()),
            VecStorage::new(b"23".to_vec()),
            VecStorage::new(b"456".to_vec()),
            VecStorage::new(b"7890".to_vec()),
        ]);

        check_read(&storage, 0, b"");
        check_read(&storage, 0, b"1234567890");
        check_read(&storage, 1, b"2");
        check_read(&storage, 1, b"23");
        check_read(&storage, 2, b"34");
        check_read(&storage, 3, b"456");
        check_read(&storage, 3, b"4567890");
    }

    #[test]
    fn concat_2() {
        let storage = ConcatStorageN::new(vec![
            VecStorage::new(b"123".to_vec()),
            VecStorage::new(b"456".to_vec()),
        ]);

        check_read(&storage, 0, b"123456");
        check_read(&storage, 1, b"23456");
        check_read(&storage, 2, b"3456");
        check_read(&storage, 3, b"456");
        check_read(&storage, 4, b"56");
        check_read(&storage, 5, b"6");
        check_read(&storage, 6, b"");
    }
}
