use crate::hexstring::HexData;
use binrw::{BinRead, BinWrite};
use hex::FromHexError;
use snafu::Snafu;
use std::fmt::Debug;
use std::str::FromStr;

#[derive(Snafu, Debug)]
pub enum IdParseError {
    InvalidLength { expected: usize, actual: usize },
    InvalidChar { char: char, index: usize },
}

fn parse_id(s: &str, result: &mut [u8]) -> Result<(), IdParseError> {
    hex::decode_to_slice(s, result).map_err(|e| match e {
        FromHexError::InvalidHexCharacter { c, index } => {
            IdParseError::InvalidChar { char: c, index }
        }
        FromHexError::OddLength | FromHexError::InvalidStringLength => {
            IdParseError::InvalidLength {
                expected: result.len() * 2,
                actual: s.len(),
            }
        }
    })?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BinRead, BinWrite)]
pub struct TitleId(u64);

impl Debug for TitleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BinRead, BinWrite)]
pub struct NcaId(HexData<0x10>);
impl FromStr for NcaId {
    type Err = IdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = [0; 0x10];
        parse_id(s, &mut result).map(|_| NcaId(HexData(result)))
    }
}
