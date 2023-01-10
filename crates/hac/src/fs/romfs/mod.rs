use crate::fs::romfs::dictionary::RomFsDictionary;
use crate::fs::romfs::structs::{
    DirectoryRomEntry, FileRomEntry, FindPosition, RomFileInfo, RomFsHeader,
};
use crate::fs::romfs::tables::HierarchicalRomTables;
use crate::fs::storage::{ReadableStorage, ReadableStorageExt, SharedStorage, SliceStorage};
use binrw::BinRead;
use snafu::{ResultExt, Snafu};
use std::fmt::Debug;

mod dictionary;
mod structs;
mod tables;

#[derive(Snafu, Debug)]
pub enum RomFsError {
    Parse {
        source: binrw::Error,
    },
    Slice {
        source: crate::fs::storage::SliceStorageError,
    },
}

#[derive(Debug)]
pub struct RomFileSystem<S: ReadableStorage> {
    storage: SharedStorage<S>,
    table: HierarchicalRomTables,
    data_offset: u64,
}

pub type FileStorage<S> = SliceStorage<SharedStorage<S>>;

pub struct Directory<'a, S: ReadableStorage> {
    fs: &'a RomFileSystem<S>,
    name: &'a str,
    position: FindPosition,
}

pub struct File<'a, S: ReadableStorage> {
    fs: &'a RomFileSystem<S>,
    name: &'a str,
    info: RomFileInfo,
}

impl<'a, S: ReadableStorage> Directory<'a, S> {
    pub fn name(&self) -> &str {
        self.name
    }

    pub fn entries(&self) -> DirectoryIter<'a, S> {
        DirectoryIter {
            fs: self.fs,
            position: self.position,
        }
    }
}

impl<'a, S: ReadableStorage> File<'a, S> {
    pub fn name(&self) -> &str {
        self.name
    }

    pub fn storage(&self) -> Result<FileStorage<S>, RomFsError> {
        let storage = self.fs.storage.clone();
        let offset = self.info.offset + self.fs.data_offset;
        let size = self.info.size;
        SliceStorage::new(storage, offset, size).context(SliceSnafu)
    }
}

impl<'a, S: ReadableStorage> Debug for Directory<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Directory")
            .field("name", &self.name)
            .field("position", &self.position)
            .finish()
    }
}

impl<'a, S: ReadableStorage> Debug for File<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("name", &self.name)
            .field("info", &self.info)
            .finish()
    }
}

#[derive(Debug)]
pub enum Entry<'a, S: ReadableStorage> {
    Directory(Directory<'a, S>),
    File(File<'a, S>),
}

// TODO specialized iterators for "only files" and "only directories" cases
pub struct DirectoryIter<'a, S: ReadableStorage> {
    fs: &'a RomFileSystem<S>,
    position: FindPosition,
}

impl<'a, S: ReadableStorage> Iterator for DirectoryIter<'a, S> {
    type Item = Entry<'a, S>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((name, position)) = self.fs.table.next_directory(&mut self.position) {
            return Some(Entry::Directory(Directory {
                fs: self.fs,
                name,
                position,
            }));
        }
        if let Some((name, info)) = self.fs.table.next_file(&mut self.position) {
            return Some(Entry::File(File {
                fs: self.fs,
                name,
                info,
            }));
        }

        None
    }
}

impl<S: ReadableStorage> RomFileSystem<S> {
    pub fn new(storage: S) -> Result<Self, RomFsError> {
        let storage = storage.shared();
        let mut io = storage.clone().buf_read();

        let header = RomFsHeader::read(&mut io).context(ParseSnafu)?;

        let dir_hash_table = storage
            .clone()
            .slice(header.dir_hash_table_offset, header.dir_hash_table_size)
            .context(SliceSnafu)?;
        let dir_meta_table = storage
            .clone()
            .slice(header.dir_meta_table_offset, header.dir_meta_table_size)
            .context(SliceSnafu)?;
        let file_hash_table = storage
            .clone()
            .slice(header.file_hash_table_offset, header.file_hash_table_size)
            .context(SliceSnafu)?;
        let file_meta_table = storage
            .clone()
            .slice(header.file_meta_table_offset, header.file_meta_table_size)
            .context(SliceSnafu)?;

        let directories =
            RomFsDictionary::<DirectoryRomEntry>::from_storage(dir_hash_table, dir_meta_table)
                .context(ParseSnafu)?;
        let files = RomFsDictionary::<FileRomEntry>::from_storage(file_hash_table, file_meta_table)
            .context(ParseSnafu)?;

        let table = HierarchicalRomTables::new(files, directories);

        Ok(Self {
            storage,
            table,
            data_offset: header.data_offset,
        })
    }

    pub fn root(&self) -> Directory<S> {
        let (name, position) = self.table.get_directory("/").unwrap();

        Directory {
            fs: self,
            name,
            position,
        }
    }

    pub fn open_directory(&self, path: &str) -> Option<Directory<S>> {
        let (name, position) = self.table.get_directory(path)?;

        Some(Directory {
            fs: self,
            name,
            position,
        })
    }

    pub fn open_file(&self, path: &str) -> Option<File<S>> {
        let (name, info) = self.table.get_file(path)?;

        Some(File {
            fs: self,
            name,
            info,
        })
    }
}
