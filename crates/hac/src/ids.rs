use crate::hexstring::HexData;
use binrw::{BinRead, BinWrite};
use hex::FromHexError;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use std::fmt::{Debug, Display};
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

macro_rules! define_program_subid {
    ($ty:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BinRead, BinWrite)]
        pub struct $ty(u64);

        impl Debug for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:016X}", self.0)
            }
        }
        impl Display for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                Debug::fmt(self, f)
            }
        }

        impl From<$ty> for ProgramId {
            fn from(id: $ty) -> Self {
                ProgramId(id.0)
            }
        }
    };
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BinRead, BinWrite)]
pub struct ProgramId(u64);
impl Debug for ProgramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016X}", self.0)
    }
}
impl Display for ProgramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

define_program_subid!(ApplicationId);
define_program_subid!(PatchId);
define_program_subid!(AddOnContentId);
define_program_subid!(DeltaId);
define_program_subid!(DataId);
define_program_subid!(DataPatchId);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BinRead, BinWrite)]
pub struct ContentId([u8; 0x10]);

// wanna lowercase, hence the separate type
impl Debug for ContentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}
impl Display for ContentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl FromStr for ContentId {
    type Err = IdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = [0; 0x10];
        parse_id(s, &mut result).map(|_| ContentId(result))
    }
}

/// Identifies a title key in the keyset.
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

impl RightsId {
    pub fn is_empty(&self) -> bool {
        self.0 .0.iter().all(|&x| x == 0)
    }
}

impl Display for RightsId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for RightsId {
    type Err = IdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = [0; 0x10];
        parse_id(s, &mut result).map(|_| RightsId(HexData(result)))
    }
}
