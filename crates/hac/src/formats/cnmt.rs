use crate::hexstring::HexData;
use crate::types::TitleId;
use binrw::{BinRead, BinWrite};
use bitflags::bitflags;
use std::io::SeekFrom;

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum ContentMetaType {
    SystemProgram = 1,
    SystemData = 2,
    SystemUpdate = 3,
    BootImagePackage = 4,
    BootImagePackageSafe = 5,
    Application = 0x80,
    Patch = 0x81,
    AddOnContent = 0x82,
    Delta = 0x83,
}

bitflags! {
    #[derive(BinRead, BinWrite)]
    pub struct ContentMetaAttribute: u8 {
        const INCLUDES_EXFAT_DRIVER = 0x01;
        const REBOOTLESS = 0x02;
        const COMPACTED = 0x04;
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
#[br(import(meta_type: ContentMetaType))]
pub enum TypeSpecificContentMeta {
    #[br(pre_assert(meta_type == ContentMetaType::Application))]
    Application {
        patch_title_id: TitleId,
        minimum_system_version: u32,
    },
    #[br(pre_assert(meta_type == ContentMetaType::Patch))]
    Patch {
        application_title_id: TitleId,
        minimum_system_version: u32,
    },
    #[br(pre_assert(meta_type == ContentMetaType::AddOnContent))]
    AddOnContent {
        application_title_id: TitleId,
        minimum_system_version: u32,
    },
    None,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum ContentType {
    Meta = 0,
    Program = 1,
    Data = 2,
    Control = 3,
    HtmlDocument = 4,
    LegalInformation = 5,
    DeltaFragment = 6,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct CnmtContentEntry {
    pub hash: HexData<0x20>,
    pub nca_id: HexData<0x10>,
    #[br(parse_with = crate::brw_utils::read_u48)]
    #[bw(write_with = crate::brw_utils::write_u48)]
    pub size: u64,
    #[brw(pad_after = 1)]
    pub ty: ContentType,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct CnmtContentMetaEntry {
    pub title_id: TitleId,
    pub version: u32,
    #[brw(pad_after = 3)]
    pub ty: ContentType,
}

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[br(import(ty: ContentMetaType, content_entry_count: u16, meta_entry_count: u16))]
pub struct CnmtMetaTables {
    #[br(count = content_entry_count)]
    pub content_entries: Vec<CnmtContentEntry>,
    #[br(count = meta_entry_count)]
    pub meta_entries: Vec<CnmtContentMetaEntry>,
    #[br(if(ty == ContentMetaType::Patch))]
    pub extended_data: Option<CnmtExtended>,
    pub hash: HexData<0x20>,
}

fn todo<R>(_: &mut R, _: &binrw::ReadOptions, _: ()) -> binrw::BinResult<u8> {
    todo!("TODO");
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct CnmtExtended {
    #[br(parse_with = todo)]
    todo: u8,
}

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(little)]
pub struct Cnmt {
    pub title_id: TitleId,
    pub version: u32,
    pub ty: ContentMetaType,
    pub field_d: u8,
    pub table_offset: u16,
    pub content_entry_count: u16,
    pub meta_entry_count: u16,
    #[brw(pad_after = 0xb)]
    pub content_meta_attributes: ContentMetaAttribute,
    #[br(args(ty))]
    pub type_specific: TypeSpecificContentMeta,

    #[br(seek_before = SeekFrom::Start(0x20 + table_offset as u64))]
    #[br(args(ty, content_entry_count, meta_entry_count))]
    pub meta_tables: CnmtMetaTables,
}
