use crate::storage::{ReadableStorage, StorageError};
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub struct SharedStorage<S: ReadableStorage> {
    storage: Arc<S>,
}

impl<S: ReadableStorage> SharedStorage<S> {
    pub fn new(storage: S) -> Self {
        Self {
            storage: Arc::new(storage),
        }
    }
}

impl<S: ReadableStorage> Deref for SharedStorage<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.storage
    }
}

impl<S: ReadableStorage> Clone for SharedStorage<S> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
        }
    }
}

impl<S: ReadableStorage> ReadableStorage for SharedStorage<S> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        self.storage.read(offset, buf)
    }

    fn get_size(&self) -> u64 {
        self.storage.get_size()
    }
}

// no writing, as:
// - aliased mutability - bad
// - checking it in runtime is complex (though it is possible, just not implemented for now)
// I want to achieve smth similar to the bytes::Bytes type
