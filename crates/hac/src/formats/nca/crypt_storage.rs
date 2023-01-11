use crate::crypto::AesKey;
use crate::hexstring::HexData;
use crate::storage::block_transforms::AesCtrBlockTransform;
use crate::storage::{
    AesCtrStorage, BlockAdapterStorage, LinearAdapterStorage, ReadableStorage, Storage,
    StorageError,
};

#[derive(Debug)]
pub enum NcaCryptStorage<S: ReadableStorage> {
    Plaintext(S),
    AesCtr(LinearAdapterStorage<AesCtrStorage<BlockAdapterStorage<S>>>),
}

impl<S: ReadableStorage> NcaCryptStorage<S> {
    pub fn new_plaintext(storage: S) -> Self {
        Self::Plaintext(storage)
    }

    pub fn new_ctr(storage: S, key: AesKey, upper_counter: u64, start_offset: u64) -> Self {
        // base nonce: first 8 bytes are specified in the fs header, the rest is big-endian offset in the section counter in AES blocks
        // the section decryptor itself will add the inner offset
        let mut nonce = [0; 0x10];
        nonce[..8].copy_from_slice(&upper_counter.to_be_bytes());
        nonce[8..].copy_from_slice(&(start_offset / 16).to_be_bytes());

        let block_adapter = BlockAdapterStorage::new(storage, 0x10);
        let transform = AesCtrBlockTransform::new(key, HexData(nonce));
        let aes_ctr = AesCtrStorage::new(block_adapter, transform);
        let linear_adapter = LinearAdapterStorage::new(aes_ctr);

        Self::AesCtr(linear_adapter)
    }
}

impl<S: ReadableStorage> ReadableStorage for NcaCryptStorage<S> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        match self {
            NcaCryptStorage::Plaintext(storage) => storage.read(offset, buf),
            NcaCryptStorage::AesCtr(storage) => storage.read(offset, buf),
        }
    }

    fn get_size(&self) -> u64 {
        match self {
            NcaCryptStorage::Plaintext(storage) => storage.get_size(),
            NcaCryptStorage::AesCtr(storage) => storage.get_size(),
        }
    }
}

impl<S: Storage> Storage for NcaCryptStorage<S> {
    fn write(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError> {
        match self {
            NcaCryptStorage::Plaintext(storage) => storage.write(offset, buf),
            NcaCryptStorage::AesCtr(storage) => storage.write(offset, buf),
        }
    }

    fn flush(&self) -> Result<(), StorageError> {
        match self {
            NcaCryptStorage::Plaintext(storage) => storage.flush(),
            NcaCryptStorage::AesCtr(storage) => storage.flush(),
        }
    }

    fn set_size(&self, new_size: u64) -> Result<(), StorageError> {
        match self {
            NcaCryptStorage::Plaintext(storage) => storage.set_size(new_size),
            NcaCryptStorage::AesCtr(storage) => storage.set_size(new_size),
        }
    }
}
