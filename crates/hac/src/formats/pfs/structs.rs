// TODO: HFS0 is __just__ like PFS0, we could probably support it later

use binrw::{BinRead, BinWrite};

#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
pub struct PartitionFsEntry {
    pub offset: u64,
    pub size: u64,
    #[brw(pad_after = 4)] // some reserved field
    pub string_table_offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little, magic = b"PFS0")]
pub struct PartitionFsHeader {
    pub num_files: u32,
    pub string_table_size: u32,

    #[brw(pad_before = 4)] // some reserved field
    #[br(count = num_files)]
    pub file_entries: Vec<PartitionFsEntry>,

    #[br(count = string_table_size)]
    pub string_table: Vec<u8>,
}

pub fn get_string(string_table: &[u8], offset: u32) -> String {
    let start = offset as usize;
    let end = string_table[start..]
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(string_table.len());

    std::str::from_utf8(&string_table[start..start + end])
        .expect("invalid utf8 in string table")
        .to_string()
}
