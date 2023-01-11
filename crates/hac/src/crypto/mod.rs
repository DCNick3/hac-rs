use crate::hexstring::HexData;
use aes::Aes128;
use binrw::{BinRead, BinWrite};
use cipher::generic_array::GenericArray;
use ctr::Ctr128BE;
use hex::FromHexError;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use std::str::FromStr;
use xts_mode::Xts128;

pub mod keyset;

#[derive(Snafu, Debug)]
pub enum KeyParseError {
    InvalidLength { expected: usize, actual: usize },
    InvalidChar { char: char, index: usize },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, BinRead, BinWrite)]
pub struct EncryptedAesKey(HexData<0x10>);
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, BinRead, BinWrite)]
pub struct EncryptedAesXtsKey(HexData<0x20>);

/// Represents an encrypted AES-128 title key.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TitleKey(HexData<0x10>);
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AesKey(HexData<0x10>);
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AesXtsKey(HexData<0x20>);

/// Identifies a title key.
#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Ord,
    PartialOrd,
    Serialize,
    Deserialize,
    BinRead,
    BinWrite,
)]
pub struct RightsId(HexData<0x10>);

fn parse_key(s: &str, result: &mut [u8]) -> Result<(), KeyParseError> {
    hex::decode_to_slice(s, result).map_err(|e| match e {
        FromHexError::InvalidHexCharacter { c, index } => {
            KeyParseError::InvalidChar { char: c, index }
        }
        FromHexError::OddLength | FromHexError::InvalidStringLength => {
            KeyParseError::InvalidLength {
                expected: result.len() * 2,
                actual: s.len(),
            }
        }
    })?;
    Ok(())
}

impl FromStr for AesKey {
    type Err = KeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = [0; 0x10];
        parse_key(s, &mut result).map(|_| AesKey(HexData(result)))
    }
}

impl FromStr for AesXtsKey {
    type Err = KeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = [0; 0x20];
        parse_key(s, &mut result).map(|_| AesXtsKey(HexData(result)))
    }
}

impl FromStr for TitleKey {
    type Err = KeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = [0; 0x10];
        parse_key(s, &mut result).map(|_| TitleKey(HexData(result)))
    }
}

impl FromStr for RightsId {
    type Err = KeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = [0; 0x10];
        parse_key(s, &mut result).map(|_| RightsId(HexData(result)))
    }
}

impl TitleKey {
    pub fn decrypt(&self, title_kek: AesKey) -> AesKey {
        title_kek.derive_key(&self.0 .0)
    }
}

impl From<[u8; 0x10]> for TitleKey {
    fn from(data: [u8; 0x10]) -> Self {
        TitleKey(HexData(data))
    }
}

impl RightsId {
    pub fn is_empty(&self) -> bool {
        self.0 .0.iter().all(|&x| x == 0)
    }
}

impl AesKey {
    fn derive_key(&self, source: &[u8; 0x10]) -> AesKey {
        use cipher::{BlockDecrypt, KeyInit};
        let mut newkey = *source;

        let crypter = Aes128::new(GenericArray::from_slice(&self.0 .0));
        crypter.decrypt_block(GenericArray::from_mut_slice(&mut newkey));

        AesKey(HexData(newkey))
    }

    pub fn decrypt_key(&self, source: EncryptedAesKey) -> AesKey {
        self.derive_key(&source.0 .0)
    }

    fn derive_xts_key(&self, source: &[u8; 0x20]) -> AesXtsKey {
        use cipher::{BlockDecrypt, KeyInit};
        let mut newkey = *source;

        let crypter = Aes128::new(GenericArray::from_slice(&self.0 .0));
        crypter.decrypt_block(GenericArray::from_mut_slice(&mut newkey[0x00..0x10]));
        crypter.decrypt_block(GenericArray::from_mut_slice(&mut newkey[0x10..0x20]));

        AesXtsKey(HexData(newkey))
    }

    pub fn decrypt_xts_key(&self, source: EncryptedAesXtsKey) -> AesXtsKey {
        self.derive_xts_key(&source.0 .0)
    }

    /// Decrypt blocks in CTR mode.
    pub fn decrypt_ctr(&self, buf: &mut [u8], ctr: &[u8; 0x10]) {
        use cipher::{KeyIvInit, StreamCipher};

        if buf.len() % 16 != 0 {
            panic!("Length must be multiple of sectors!")
        }

        let key = GenericArray::from_slice(&self.0 .0);
        let iv = GenericArray::from_slice(ctr);
        let mut crypter = Ctr128BE::<Aes128>::new(key, iv);
        crypter.apply_keystream(buf);
    }

    pub fn encrypt_ctr(&self, buf: &mut [u8], ctr: &[u8; 0x10]) {
        use cipher::{KeyIvInit, StreamCipher};

        if buf.len() % 16 != 0 {
            panic!("Length must be multiple of sectors!")
        }

        let key = GenericArray::from_slice(&self.0 .0);
        let iv = GenericArray::from_slice(ctr);
        let mut crypter = Ctr128BE::<Aes128>::new(key, iv);
        crypter.apply_keystream(buf);
    }
}

fn get_tweak(mut sector: usize) -> [u8; 0x10] {
    let mut tweak = [0; 0x10];
    for tweak in tweak.iter_mut().rev() {
        /* Nintendo LE custom tweak... */
        *tweak = (sector & 0xFF) as u8;
        sector >>= 8;
    }
    tweak
}

impl AesXtsKey {
    #[inline]
    fn to_crypter(&self) -> Xts128<Aes128> {
        use cipher::KeyInit;

        let key1 = Aes128::new(GenericArray::from_slice(&self.0 .0[0x00..0x10]));
        let key2 = Aes128::new(GenericArray::from_slice(&self.0 .0[0x10..0x20]));
        Xts128::<Aes128>::new(key1, key2)
    }

    pub fn decrypt(&self, data: &mut [u8], mut sector: usize, sector_size: usize) {
        if data.len() % sector_size != 0 {
            panic!("Length must be multiple of sectors!")
        }

        let crypter = self.to_crypter();

        for i in (0..data.len()).step_by(sector_size) {
            let tweak = get_tweak(sector);

            crypter.decrypt_sector(&mut data[i..i + sector_size], tweak);
            sector += 1;
        }
    }

    pub fn encrypt(&self, data: &mut [u8], mut sector: usize, sector_size: usize) {
        if data.len() % sector_size != 0 {
            panic!("Length must be multiple of sectors!")
        }

        let crypter = self.to_crypter();

        for i in (0..data.len()).step_by(sector_size) {
            let tweak = get_tweak(sector);

            crypter.decrypt_sector(&mut data[i..i + sector_size], tweak);
            sector += 1;
        }
    }
}
