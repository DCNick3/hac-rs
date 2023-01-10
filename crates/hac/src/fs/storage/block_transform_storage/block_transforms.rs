use crate::crypto::AesKey;
use crate::fs::storage::BlockTransform;
use crate::hexstring::HexData;

#[derive(Debug, Clone)]
pub struct AesCtrBlockTransform {
    key: AesKey,
    nonce: HexData<0x10>,
}

impl AesCtrBlockTransform {
    pub fn new(key: AesKey, nonce: HexData<0x10>) -> Self {
        Self { key, nonce }
    }

    fn get_ctr(&self, block_index: u64) -> [u8; 0x10] {
        (u128::from_be_bytes(self.nonce.0) + block_index as u128).to_be_bytes()
        // let mut ctr = [0; 0x10];
        // // Write section nonce in Big Endian.
        // ctr[..8].copy_from_slice(&self.nonce.0);
        // // Set ctr to offset / BLOCK_SIZE, in big endian.
        // ctr[8..].copy_from_slice(&block_index.to_be_bytes());
        // ctr
    }
}

impl BlockTransform for AesCtrBlockTransform {
    const BLOCK_SIZE: usize = 0x10;

    fn transform_read(&self, block: &mut [u8], block_index: u64) {
        debug_assert_eq!(block.len() % Self::BLOCK_SIZE, 0);

        self.key.decrypt_ctr(block, &self.get_ctr(block_index));
    }

    fn transform_write(&self, block: &mut [u8], block_index: u64) {
        debug_assert_eq!(block.len() % Self::BLOCK_SIZE, 0);

        self.key.encrypt_ctr(block, &self.get_ctr(block_index));
    }
}
