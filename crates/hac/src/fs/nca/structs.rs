use crate::crypto::{EncryptedAesKey, EncryptedAesXtsKey};
use crate::hexstring::HexData;
use binrw::{BinRead, BinWrite};
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum NcaSectionType {
    Code,
    Data,
    Logo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum NcaContentType {
    Program,
    Meta,
    Control,
    Manual,
    Data,
    PublicData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum DistributionType {
    Download,
    GameCard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum NcaEncryptionType {
    Auto,
    None,
    Xts,
    AesCtr,
    AesCtrEx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum NcaHashType {
    Auto,
    None,
    Sha256,
    Ivfc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum NcaFormatType {
    Romfs,
    Pfs0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
pub struct NcaSignature(pub HexData<0x100>);

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub enum NcaMagic {
    #[brw(magic = b"NCA0")]
    Nca0,
    #[brw(magic = b"NCA1")]
    Nca1,
    #[brw(magic = b"NCA2")]
    Nca2,
    #[brw(magic = b"NCA3")]
    Nca3,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
pub struct TitleId(u64);

impl Debug for TitleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
pub struct RightsId(pub HexData<0x10>);

#[derive(Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct SectionTableOffset(u64);

impl From<SectionTableOffset> for u64 {
    fn from(o: SectionTableOffset) -> Self {
        o.0 * 0x200
    }
}

impl From<u64> for SectionTableOffset {
    fn from(o: u64) -> Self {
        SectionTableOffset(o / 0x200)
    }
}

impl Debug for SectionTableOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0 * 0x200)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct SectionTableEntry {
    pub offset: SectionTableOffset,
    pub size: SectionTableOffset,
}

impl SectionTableEntry {
    pub fn present(&self) -> bool {
        self.offset.0 != 0 && self.size.0 != 0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
pub struct Sha256Hash(pub HexData<0x20>);

impl Sha256Hash {
    pub fn verify(&self, data: &[u8]) -> Result<(), ()> {
        use digest::Digest;
        let mut hasher = sha2::Sha256::default();
        hasher.update(data);
        let hash = hasher.finalize();
        (hash.as_ref() == self.0 .0).then_some(()).ok_or(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct NcaKeyArea {
    pub encrypted_xts_key: EncryptedAesXtsKey,
    pub encrypted_ctr_key: EncryptedAesKey,
    pub encrypted_ctr_ex_key: EncryptedAesKey,
    pub encrypted_ctr_hw_key: EncryptedAesKey,
    pub unused: HexData<0xb0>,
}

/// NCA header, corresponding to the first 0x400 bytes of the decrypted NCA
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct NcaHeader {
    pub fixed_key_signature: NcaSignature,
    pub npdm_signature: NcaSignature,
    pub magic: NcaMagic,
    pub distribution_type: DistributionType,
    pub content_type: NcaContentType,
    pub key_generation_1: u8,
    pub key_area_key_index: u8,
    pub nca_size: u64,
    pub title_id: TitleId,
    pub content_index: u32,
    pub sdk_version: u32,
    #[brw(pad_after = 0xf)]
    pub key_generation_2: u8,
    pub rights_id: RightsId,
    pub section_table: [SectionTableEntry; 4],
    pub fs_header_hashes: [Sha256Hash; 4],
    pub key_area: NcaKeyArea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
pub struct Sha256IntegrityInfoLevel {
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
pub struct Sha256IntegrityInfo {
    pub master_hash: Sha256Hash,
    pub block_size: u32,
    pub level_count: u32,
    pub level_info: [Sha256IntegrityInfoLevel; 6],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
pub struct IvfcIntegrityInfoLevel {
    pub offset: u64,
    pub size: u64,
    pub block_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[brw(magic = b"IVFC")]
pub struct IvfcIntegrityInfo {
    pub version: u32,
    pub master_hash_size: u32,
    pub level_count: u32,
    pub level_info: [IvfcIntegrityInfoLevel; 6],
    pub salt_source: HexData<0x20>,
    pub master_hash: HexData<0x38>, // this is the max size of the hash
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[br(import(hash_type: NcaHashType))]
pub enum IntegrityInfo {
    #[br(pre_assert(hash_type == NcaHashType::None))]
    None,
    #[br(pre_assert(hash_type == NcaHashType::Sha256))]
    Sha256(Sha256IntegrityInfo),
    #[br(pre_assert(hash_type == NcaHashType::Ivfc))]
    Ivfc(IvfcIntegrityInfo),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
pub struct PatchInfo {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
pub struct SparseInfo {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
pub struct CompressionInfo {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct NcaFsHeader {
    pub version: u16,
    pub format_type: NcaFormatType,
    pub hash_type: NcaHashType,
    pub encryption_type: NcaEncryptionType,

    #[brw(pad_before = 0x3)]
    #[br(args(hash_type))]
    #[brw(pad_size_to = 0xf8)]
    pub integrity_info: IntegrityInfo,

    #[brw(pad_size_to = 0x40)]
    pub patch_info: PatchInfo,

    pub upper_counter: u64,

    #[brw(pad_size_to = 0x30)]
    pub sparse_info: SparseInfo,

    #[brw(pad_size_to = 0x28)] // this is the allocated size for CompressionInfo
    #[brw(pad_after = 0x60)] // this is unused space after it
    pub compression_info: CompressionInfo,
}
