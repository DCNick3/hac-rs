use crate::formats::romfs::dictionary::{RomEntryKey, RomFsDictionary};
use crate::formats::romfs::structs::{
    DirectoryRomEntry, FileRomEntry, FindPosition, RomFileInfo, RomId,
};

#[derive(Debug)]
pub struct HierarchicalRomTables {
    file_table: RomFsDictionary<FileRomEntry>,
    directory_table: RomFsDictionary<DirectoryRomEntry>,
}

impl HierarchicalRomTables {
    pub fn new(
        file_table: RomFsDictionary<FileRomEntry>,
        directory_table: RomFsDictionary<DirectoryRomEntry>,
    ) -> Self {
        Self {
            file_table,
            directory_table,
        }
    }

    fn find_path_recursive<'a>(&self, path: &'a str) -> Option<RomEntryKey<'a>> {
        let path = path.split('/');
        let mut key = RomEntryKey {
            name: "",
            parent: RomId(0),
        };

        for part in path {
            key.name = part;
            (_, key.parent) = self.directory_table.get_offset_from_key(key)?;
        }

        Some(key)
    }

    pub fn get_file(&self, path: &str) -> Option<(&str, RomFileInfo)> {
        let key = self.find_path_recursive(path)?;

        self.file_table
            .get_entry_by_key(key)
            .map(|(name, v)| (name, v.value.info))
    }

    pub fn get_directory(&self, path: &str) -> Option<(&str, FindPosition)> {
        let key = self.find_path_recursive(path)?;

        self.directory_table
            .get_entry_by_key(key)
            .map(|(name, v)| (name, v.value.position))
    }

    pub fn next_file(&self, position: &mut FindPosition) -> Option<(&str, RomFileInfo)> {
        if position.next_file.is_none() {
            return None;
        }

        let (name, entry) = self.file_table.get_entry_by_id(position.next_file);

        position.next_file = entry.value.next_sibling;

        Some((name, entry.value.info))
    }

    pub fn next_directory(&self, position: &mut FindPosition) -> Option<(&str, FindPosition)> {
        if position.next_directory.is_none() {
            return None;
        }

        let (name, entry) = self
            .directory_table
            .get_entry_by_id(position.next_directory);

        position.next_directory = entry.value.next_sibling;

        Some((name, entry.value.position))
    }
}
