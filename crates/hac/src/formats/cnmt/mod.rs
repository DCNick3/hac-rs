use crate::hexstring::HexData;
use crate::ids::{AnyId, ApplicationId, ContentId, DataPatchId, PatchId};
use crate::version::Version;
use binrw::{BinRead, BinWrite};
use bitflags::bitflags;
use std::io::SeekFrom;

pub mod patch_meta_extended_data;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, BinRead, BinWrite)]
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum ContentInstallType {
    Full = 0,
    FragmentOnly = 1,
    // Unknown = 7,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, BinRead, BinWrite)]
pub struct ContentMetaKey {
    pub id: AnyId,
    pub version: Version,
    pub ty: ContentMetaType,
    #[brw(pad_after = 2)]
    pub install_ty: ContentInstallType,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct ContentMetaAttribute(u8);
bitflags! {
    impl ContentMetaAttribute: u8 {
        const INCLUDES_EXFAT_DRIVER = 0x01;
        const REBOOTLESS = 0x02;
        const COMPACTED = 0x04;
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct ContentMetaInstallState(u8);
bitflags! {
    impl ContentMetaInstallState: u8 {
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
        required_system_version: Version,
        required_application_version: Version,
    },
    #[br(pre_assert(meta_type == ContentMetaType::Patch))]
    Patch {
        application_id: ApplicationId,
        required_system_version: Version,
        #[brw(pad_after = 8)]
        extended_data_size: u32,
    },
    #[br(pre_assert(meta_type == ContentMetaType::AddOnContent))]
    AddOnContent {
        application_id: ApplicationId,
        required_application_version: Version,
        #[brw(pad_after = 3)]
        content_accessibilities: u8,
        data_patch_id: DataPatchId,
    },
    #[br(pre_assert(meta_type == ContentMetaType::Delta))]
    Delta {
        application_id: ApplicationId,
        #[brw(pad_after = 4)]
        extended_data_size: u32,
    },
    None,
}

impl ExtendedMetaHeader {
    pub fn extended_data_size(&self) -> u32 {
        match *self {
            ExtendedMetaHeader::SystemUpdate { extended_data_size }
            | ExtendedMetaHeader::Patch {
                extended_data_size, ..
            }
            | ExtendedMetaHeader::Delta {
                extended_data_size, ..
            } => extended_data_size,
            ExtendedMetaHeader::Application { .. }
            | ExtendedMetaHeader::AddOnContent { .. }
            | ExtendedMetaHeader::None => 0,
        }
    }
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
    pub id: ContentId,
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
    pub title_id: AnyId,
    pub version: Version,
    pub ty: NcmContentType,
    #[brw(pad_after = 2)]
    pub attributes: ContentMetaAttribute,
}

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[br(import(meta_type: ContentMetaType, extended_data_size: u32))]
pub enum ExtendedData {
    #[br(pre_assert(extended_data_size != 0 && meta_type == ContentMetaType::Patch))]
    Patch(patch_meta_extended_data::PatchMetaExtendedData),
    #[br(pre_assert(extended_data_size == 0))]
    None,
}

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(little)]
pub struct PackagedContentMeta {
    pub id: AnyId,
    pub version: Version,
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
    pub required_download_system_version: Version,

    #[brw(pad_before = 4)]
    #[br(args(ty, extended_header_size))]
    pub extended_header: ExtendedMetaHeader,

    #[br(seek_before = SeekFrom::Start(0x20 + extended_header_size as u64))]
    #[br(count = content_count)]
    pub content_info: Vec<PackagedContentInfo>,
    #[br(count = content_meta_count)]
    pub content_meta_info: Vec<ContentMetaInfo>,
    // TODO: be more robust by checking/enforcing extended data size (from extended header)
    #[br(args(ty, extended_header.extended_data_size()))]
    pub extended_data: ExtendedData,

    pub hash: HexData<0x20>,
}

impl PackagedContentMeta {
    pub fn content_meta_key(&self) -> ContentMetaKey {
        ContentMetaKey {
            id: self.id,
            version: self.version,
            ty: self.ty,
            install_ty: self.content_install_type,
        }
    }
}
