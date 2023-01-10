mod structs;

use crate::fs::pfs::structs::{get_string, PartitionFsHeader};
use crate::fs::storage::{
    ReadableStorage, ReadableStorageExt, SharedStorage, SliceStorage, SliceStorageError,
};
use binrw::BinRead;
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Snafu, Debug)]
pub struct PfsParseError {
    source: binrw::Error,
}

#[derive(Snafu, Debug)]
pub struct PfsOpenError {
    source: SliceStorageError,
}

#[derive(Debug, Copy, Clone)]
struct FileInfo {
    offset: u64,
    size: u64,
}

pub struct PartitionFileSystem<S: ReadableStorage> {
    storage: SharedStorage<S>,
    files: HashMap<String, FileInfo>,
}

pub type FileStorage<S> = SliceStorage<SharedStorage<S>>;

pub struct PartitionFileSystemFile<'a, S: ReadableStorage> {
    fs: &'a PartitionFileSystem<S>,
    filename: &'a str,
    info: FileInfo,
}

impl<'a, S: ReadableStorage> PartitionFileSystemFile<'a, S> {
    pub fn filename(&self) -> &str {
        self.filename
    }

    pub fn storage(&self) -> Result<FileStorage<S>, PfsOpenError> {
        let storage = self.fs.storage.clone();
        let offset = self.info.offset;
        let size = self.info.size;
        SliceStorage::new(storage, offset, size).context(PfsOpenSnafu)
    }
}

impl<'a, S: ReadableStorage> Debug for PartitionFileSystemFile<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartitionFileSystemFile")
            .field("filename", &self.filename)
            .field("offset", &self.info.offset)
            .field("size", &self.info.size)
            .finish()
    }
}

pub struct PartitionFileSystemIter<'a, S: ReadableStorage> {
    fs: &'a PartitionFileSystem<S>,
    iter: std::collections::hash_map::Iter<'a, String, FileInfo>,
}

impl<'a, S: ReadableStorage> Iterator for PartitionFileSystemIter<'a, S> {
    type Item = PartitionFileSystemFile<'a, S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(filename, &info)| PartitionFileSystemFile {
                fs: self.fs,
                filename: filename.as_str(),
                info,
            })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }
}

impl<S: ReadableStorage> PartitionFileSystem<S> {
    pub fn new(storage: S) -> Result<Self, PfsParseError> {
        let mut io = storage.buf_read();

        let PartitionFsHeader {
            file_entries,
            string_table,
            ..
        } = PartitionFsHeader::read(&mut io).context(PfsParseSnafu)?;

        let files = file_entries
            .into_iter()
            .map(|e| {
                let name = get_string(&string_table, e.string_table_offset);
                let file = FileInfo {
                    offset: e.offset,
                    size: e.size,
                };
                (name, file)
            })
            .collect();

        let storage = io.into_inner().into_inner().shared();
        Ok(Self { storage, files })
    }

    pub fn get_file(&self, path: &str) -> Option<PartitionFileSystemFile<S>> {
        self.files
            .get_key_value(path)
            .map(|(filename, &info)| PartitionFileSystemFile {
                fs: self,
                filename: filename.as_str(),
                info,
            })
    }

    pub fn get_file_storage(&self, path: &str) -> Result<Option<FileStorage<S>>, PfsOpenError> {
        self.get_file(path).map(|file| file.storage()).transpose()
    }

    pub fn iter(&self) -> PartitionFileSystemIter<S> {
        PartitionFileSystemIter {
            fs: self,
            iter: self.files.iter(),
        }
    }
}
