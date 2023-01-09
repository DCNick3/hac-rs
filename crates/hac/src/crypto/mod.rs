use crate::impl_debug_deserialize_serialize_hexstring;
use hex::FromHexError;
use snafu::Snafu;
use std::str::FromStr;

pub mod keyset;

#[derive(Snafu, Debug)]
pub enum KeyParseError {
    InvalidLength { expected: usize, actual: usize },
    InvalidChar { char: char, index: usize },
}

#[derive(Copy, Clone)]
pub struct AesKey([u8; 0x10]);
#[derive(Copy, Clone)]
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

impl_debug_deserialize_serialize_hexstring!(AesKey);
