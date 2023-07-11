use binrw::{BinRead, BinWrite};

#[derive(Debug, Copy, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct RomFsHeader {
    // NOTE: no support for pre-release RomFs
    // LibHac: "Old pre-release romfs is exactly the same except the fields in the header are 32-bit instead of 64-bit"
    pub header_size: u64,
    pub dir_hash_table_offset: u64,
    pub dir_hash_table_size: u64,
    pub dir_meta_table_offset: u64,
    pub dir_meta_table_size: u64,
    pub file_hash_table_offset: u64,
    pub file_hash_table_size: u64,
    pub file_meta_table_offset: u64,
    pub file_meta_table_size: u64,
    pub data_offset: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, BinRead, BinWrite)]
pub struct RomId(pub i32);

impl RomId {
    pub const NONE: Self = Self(-1);

    pub fn is_none(&self) -> bool {
        *self == Self::NONE
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct RomFsEntry<T: for<'a> BinRead<Args<'a> = ()> + for<'a> BinWrite<Args<'a> = ()> + 'static>
{
    pub parent: RomId,
    pub value: T,
    pub next: RomId,
    pub key_length: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, BinRead, BinWrite)]
pub struct FindPosition {
    pub next_directory: RomId,
    pub next_file: RomId,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, BinRead, BinWrite)]
pub struct DirectoryRomEntry {
    pub next_sibling: RomId,
    pub position: FindPosition,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, BinRead, BinWrite)]
pub struct RomFileInfo {
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, BinRead, BinWrite)]
pub struct FileRomEntry {
    pub next_sibling: RomId,
    pub info: RomFileInfo,
}
