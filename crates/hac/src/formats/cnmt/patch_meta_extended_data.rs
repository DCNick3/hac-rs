use crate::formats::cnmt::{
    ContentInfo, ContentMetaKey, ContentMetaType, NcmContentType, PackagedContentInfo, UpdateType,
};
use crate::hexstring::HexData;
use crate::ids::{ContentId, PatchId, ProgramId};
use binrw::{BinRead, BinWrite};

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct PatchHistoryHeader {
    pub key: ContentMetaKey,
    pub hash: HexData<0x20>,
    pub content_count: u16,
    pub field_32: u16,
    pub field_34: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct PatchDeltaHistory {
    pub title_id_old: PatchId,
    pub title_id_new: PatchId,
    pub version_old: u32,
    pub version_new: u32,
    #[brw(pad_after = 0x8)]
    pub download_size: u64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct PatchDeltaHeader {
    pub source_id: PatchId,
    pub destination_id: PatchId,
    pub source_version: u32,
    pub destination_version: u32,
    #[brw(pad_after = 0x6)]
    pub fragment_set_count: u16,
    #[brw(pad_after = 0x6)]
    pub content_count: u16,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct FragmentSet {
    pub source_content_id: ContentId,
    pub destination_content_id: ContentId,
    #[br(parse_with = crate::brw_utils::read_u48)]
    #[bw(write_with = crate::brw_utils::write_u48)]
    pub source_size: u64,
    // WHY ARE YOU REVERSED???
    #[br(parse_with = crate::brw_utils::read_u48_rev)]
    #[bw(write_with = crate::brw_utils::write_u48_rev)]
    pub destination_size: u64,
    pub fragment_count: u16,
    pub target_content_type: NcmContentType,
    #[brw(pad_after = 0x4)]
    pub update_type: UpdateType,
}

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct FragmentIndicator {
    pub content_index: u16,
    pub fragment_index: u16,
}

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct PatchMetaExtendedData {
    pub history_count: u32,
    pub delta_history_count: u32,
    pub delta_count: u32,
    pub fragment_set_count: u32,
    pub history_content_total_count: u32,
    pub delta_content_total_count: u32,

    #[brw(pad_before = 0x4)]
    #[br(count = history_count)]
    pub patch_history: Vec<PatchHistoryHeader>,
    #[br(count = delta_history_count)]
    pub patch_delta_history: Vec<PatchDeltaHistory>,
    #[br(count = delta_count)]
    pub patch_delta_headers: Vec<PatchDeltaHeader>,
    #[br(count = fragment_set_count)]
    pub fragment_sets: Vec<FragmentSet>,
    #[br(count = history_content_total_count)]
    pub history_content: Vec<ContentInfo>,
    #[br(count = delta_content_total_count)]
    pub delta_contents: Vec<PackagedContentInfo>,

    #[br(count = fragment_sets.iter().map(|x| x.fragment_count as usize).sum::<usize>())]
    pub fragment_indicators: Vec<FragmentIndicator>,
}
