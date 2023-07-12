use binrw::{BinRead, BinWrite};
use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, BinRead, BinWrite)]
pub struct Version(u32);

impl Version {
    pub fn into_parts(self) -> (u8, u8, u8) {
        let [v0, v1, v2, _] = self.0.to_be_bytes();

        (v0 + 1, v1, v2)
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (v0, v1, v2) = self.into_parts();

        f.debug_tuple("Version")
            .field(&v0)
            .field(&v1)
            .field(&v2)
            .finish()
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (v0, v1, v2) = self.into_parts();

        write!(f, "{}.{}.{}", v0, v1, v2)
    }
}

impl From<u32> for Version {
    fn from(v: u32) -> Self {
        Self(v)
    }
}
impl From<Version> for u32 {
    fn from(v: Version) -> Self {
        v.0
    }
}
