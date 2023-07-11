use crate::hexstring::HexData;
use crate::ids::{ApplicationId, ContentId, PatchId, ProgramId};
use binrw::{BinRead, BinWrite};
use bitflags::bitflags;
use std::io::SeekFrom;

pub mod patch_meta_extended_data;

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum ContentMetaType {
    // Unknown = 0,
    SystemProgram = 1,
    SystemData = 2,
    SystemUpdate = 3,
    BootImagePackage = 4,
    BootImagePackageSafe = 5,
    /// A user program (commonly know as base game / app)
    Application = 0x80,
    /// A patch for a user program (commonly known as update)
    Patch = 0x81,
    /// DLC for a user program
    AddOnContent = 0x82,
    Delta = 0x83,
    DataPatch = 0x84,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum StorageId {
    None = 0,
    Host = 1,
    GameCard = 2,
    BuiltInSystem = 3,
    BuiltInUser = 4,
    SdCard = 5,
    Any = 6,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum ContentInstallType {
    Full = 0,
    FragmentOnly = 1,
    // Unknown = 7,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct ContentMetaKey {
    pub title_id: ProgramId,
    pub version: u32,
    pub ty: ContentMetaType,
    #[brw(pad_after = 2)]
    pub install_ty: ContentInstallType,
}

bitflags! {
    #[derive(BinRead, BinWrite)]
    pub struct ContentMetaAttribute: u8 {
        const INCLUDES_EXFAT_DRIVER = 0x01;
        const REBOOTLESS = 0x02;
        const COMPACTED = 0x04;
    }
}

bitflags! {
    #[derive(BinRead, BinWrite)]
    pub struct ContentMetaInstallState: u8 {
        const COMMITTED = 0x01;
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
#[br(import(meta_type: ContentMetaType, extended_header_size: u16))]
pub enum ExtendedMetaHeader {
    #[br(pre_assert(meta_type == ContentMetaType::SystemUpdate && extended_header_size != 0))]
    SystemUpdate {
        extended_data_size: u32,
    },
    #[br(pre_assert(meta_type == ContentMetaType::Application))]
    Application {
        patch_id: PatchId,
        required_system_version: u32,
        required_application_version: u32,
    },
    #[br(pre_assert(meta_type == ContentMetaType::Patch))]
    Patch {
        application_id: ApplicationId,
        required_system_version: u32,
        #[brw(pad_after = 8)]
        extended_data_size: u32,
    },
    #[br(pre_assert(meta_type == ContentMetaType::AddOnContent))]
    AddOnContent {
        application_id: ApplicationId,
        required_application_version: u32,
        content_accessibilities: u8,
    },
    #[br(pre_assert(meta_type == ContentMetaType::Delta))]
    Delta {
        application_id: ApplicationId,
        #[brw(pad_after = 4)]
        extended_data_size: u32,
    },
    None,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum NcmContentType {
    Meta = 0,
    Program = 1,
    Data = 2,
    Control = 3,
    HtmlDocument = 4,
    LegalInformation = 5,
    DeltaFragment = 6,
}

// pub struct Digest {}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct ContentInfo {
    pub content_id: ContentId,
    #[br(parse_with = crate::brw_utils::read_u40)]
    #[bw(write_with = crate::brw_utils::write_u40)]
    pub size: u64,
    pub content_attributes: u8,
    pub ty: NcmContentType,
    pub id_offset: u8,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct PackagedContentInfo {
    pub hash: HexData<0x20>,
    pub content_info: ContentInfo,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct ContentMetaInfo {
    pub title_id: ProgramId,
    pub version: u32,
    pub ty: NcmContentType,
    #[brw(pad_after = 2)]
    pub attributes: ContentMetaAttribute,
}

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[br(import(meta_type: ContentMetaType))]
pub enum ExtendedData {
    #[br(pre_assert(meta_type == ContentMetaType::Patch))]
    Patch(patch_meta_extended_data::PatchMetaExtendedData),
    #[br(pre_assert(meta_type == ContentMetaType::Application))]
    None,
}

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(little)]
pub struct PackagedContentMeta {
    pub title_id: ProgramId,
    pub version: u32,
    pub ty: ContentMetaType,
    pub field_d: u8,
    /// Must match the size from the extended header struct for this content meta type (SystemUpdate, Application, Patch, AddOnContent, Delta).
    pub extended_header_size: u16,
    /// Determines how many PackagedContentInfo entries are available after the extended header.
    pub content_count: u16,
    /// Determines how many ContentMetaInfo entries are available after the PackagedContentInfo entries. Only used for SystemUpdate.
    pub content_meta_count: u16,
    pub attributes: ContentMetaAttribute,
    pub storage_id: StorageId,
    pub content_install_type: ContentInstallType,
    pub install_state: ContentMetaInstallState,
    pub required_download_system_version: u32,

    #[brw(pad_before = 4)]
    #[br(args(ty, extended_header_size))]
    pub extended_header: ExtendedMetaHeader,

    #[br(seek_before = SeekFrom::Start(0x20 + extended_header_size as u64))]
    #[br(count = content_count)]
    pub content_info: Vec<PackagedContentInfo>,
    #[br(count = content_meta_count)]
    pub content_meta_info: Vec<ContentMetaInfo>,
    // TODO: be more robust by checking/enforcing extended data size (from extended header)
    #[br(args(ty))]
    pub extended_data: ExtendedData,

    pub hash: HexData<0x20>,
}
