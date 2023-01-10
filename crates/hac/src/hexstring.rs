use binrw::{BinRead, BinWrite};

struct Hexstring<'a>(pub &'a [u8]);

impl<'a> core::fmt::Debug for Hexstring<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for byte in self.0 {
            write!(f, "{:02X}", byte)?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, BinRead, BinWrite)]
pub struct HexData<const N: usize>(pub [u8; N]);

impl<const N: usize> core::fmt::Debug for HexData<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", Hexstring(&self.0[..]))
    }
}

impl<const N: usize> core::fmt::Display for HexData<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

impl<'de, const N: usize> serde::Deserialize<'de> for HexData<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct StrVisitor<const N: usize>;
        impl<'de, const N: usize> serde::de::Visitor<'de> for StrVisitor<N> {
            type Value = HexData<N>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a character hexstring")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut value = [0; N];
                if s.len() != N * 2 {
                    return Err(E::invalid_length(s.len(), &self));
                }
                for (idx, c) in s.bytes().enumerate() {
                    let c = match c {
                        b'a'..=b'z' => c - b'a' + 10,
                        b'A'..=b'Z' => c - b'A' + 10,
                        b'0'..=b'9' => c - b'0',
                        _ => return Err(E::invalid_value(serde::de::Unexpected::Str(s), &self)),
                    };
                    value[idx / 2] |= c << if idx % 2 == 0 { 4 } else { 0 }
                }

                Ok(HexData(value))
            }
        }

        deserializer.deserialize_str(StrVisitor)
    }
}

impl<const N: usize> serde::Serialize for HexData<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<const N: usize> AsRef<[u8]> for HexData<N> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<const N: usize> AsMut<[u8]> for HexData<N> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl<const N: usize> From<[u8; N]> for HexData<N> {
    fn from(value: [u8; N]) -> Self {
        Self(value)
    }
}

impl<const N: usize> From<HexData<N>> for [u8; N] {
    fn from(value: HexData<N>) -> Self {
        value.0
    }
}
