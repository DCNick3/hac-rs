use crate::hexstring::HexData;
use aes::Aes128;
use binrw::{BinRead, BinWrite};
use cipher::generic_array::GenericArray;
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AesKey([u8; 0x10]);
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AesXtsKey([u8; 0x20]);

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
        parse_key(s, &mut result).map(|_| AesKey(result))
    }
}

impl FromStr for AesXtsKey {
    type Err = KeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = [0; 0x20];
        parse_key(s, &mut result).map(|_| AesXtsKey(result))
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

        let key1 = Aes128::new(GenericArray::from_slice(&self.0[0x00..0x10]));
        let key2 = Aes128::new(GenericArray::from_slice(&self.0[0x10..0x20]));
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
