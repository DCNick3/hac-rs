use crate::hexstring::HexData;
use crate::ids::{NcaId, TitleId};
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
#[brw(repr = u8)]
pub enum UpdateType {
    ApplyAsDelta = 0,
    Overwrite = 1,
    Create = 2,
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
    pub nca_id: NcaId,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct CnmtPrevMetaEntry {
    pub title_id: TitleId,
    pub version: u32,
    pub ty: ContentMetaType,
    #[brw(pad_before = 0x3)]
    pub hash: HexData<0x20>,
    pub content_count: u16,
    pub field_32: u16,
    pub field_34: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct CnmtPrevDelta {
    pub title_id_old: TitleId,
    pub title_id_new: TitleId,
    pub version_old: u32,
    pub version_new: u32,
    pub size: u64,
    pub field_20: u64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct CnmtDeltaSetInfo {
    pub title_id_old: TitleId,
    pub title_id_new: TitleId,
    pub version_old: u32,
    pub version_new: u32,
    pub fragment_set_count: u64,
    pub delta_content_count: u64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct CnmtFragmentSetInfo {
    pub nca_id_old: NcaId,
    pub nca_id_new: NcaId,
    #[br(parse_with = crate::brw_utils::read_u48)]
    #[bw(write_with = crate::brw_utils::write_u48)]
    pub size_old: u64,
    // WHY ARE YOU REVERSED???
    #[br(parse_with = crate::brw_utils::read_u48_rev)]
    #[bw(write_with = crate::brw_utils::write_u48_rev)]
    pub size_new: u64,
    pub fragment_count: u16,
    pub ty: ContentType,
    pub update_type: UpdateType,
    pub field_30: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct CnmtPrevContent {
    pub nca_id: NcaId,
    #[br(parse_with = crate::brw_utils::read_u48)]
    #[bw(write_with = crate::brw_utils::write_u48)]
    pub size: u64,
    #[brw(pad_after = 0x1)]
    pub ty: ContentType,
}

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct FragmentMapEntry {
    pub content_index: u16,
    pub fragment_index: u16,
}

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct CnmtExtended {
    pub prev_meta_count: u32,
    pub prev_delta_set_count: u32,
    pub delta_set_count: u32,
    pub fragment_set_count: u32,
    pub prev_content_count: u32,
    pub delta_content_count: u32,

    #[brw(pad_before = 0x4)]
    #[br(count = prev_meta_count)]
    pub prev_metas: Vec<CnmtPrevMetaEntry>,
    #[br(count = prev_delta_set_count)]
    pub prev_deltas: Vec<CnmtPrevDelta>,
    #[br(count = delta_set_count)]
    pub delta_sets: Vec<CnmtDeltaSetInfo>,
    #[br(count = fragment_set_count)]
    pub fragment_sets: Vec<CnmtFragmentSetInfo>,
    #[br(count = prev_content_count)]
    pub prev_contents: Vec<CnmtPrevContent>,
    #[br(count = delta_content_count)]
    pub delta_contents: Vec<CnmtContentEntry>,

    #[br(count = fragment_sets.iter().map(|x| x.fragment_count as usize).sum::<usize>())]
    pub fragment_map: Vec<FragmentMapEntry>,
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
