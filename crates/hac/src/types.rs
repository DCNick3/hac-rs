use crate::hexstring::HexData;
use binrw::{BinRead, BinWrite};
use std::fmt::Debug;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BinRead, BinWrite)]
pub struct TitleId(u64);

impl Debug for TitleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BinRead, BinWrite)]
pub struct NcaId(HexData<0x10>);
