mod structs;

use crate::fs::filesystem::{Entry, ReadableDirectory, ReadableFile, ReadableFileSystem};
use crate::fs::pfs::structs::{get_string, PartitionFsHeader};
use crate::fs::storage::{
    ReadableStorage, ReadableStorageExt, SharedStorage, SliceStorage, SliceStorageError,
};
use binrw::BinRead;
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Seek;

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

#[derive(Debug)]
pub struct PartitionFileSystem<S: ReadableStorage> {
    storage: SharedStorage<S>,
    files: HashMap<String, FileInfo>,
    header_size: u64,
}

pub type FileStorage<S> = SliceStorage<SharedStorage<S>>;

// this directory is kinda fake, the PFS is flat
// so, this directory is always the root directory
pub struct Directory<'a, S: ReadableStorage> {
    fs: &'a PartitionFileSystem<S>,
}

pub struct File<'a, S: ReadableStorage> {
    fs: &'a PartitionFileSystem<S>,
    filename: &'a str,
    info: FileInfo,
}

impl<'a, S: ReadableStorage> Debug for Directory<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Directory").finish()
    }
}

impl<'a, S: ReadableStorage> Debug for File<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartitionFileSystemFile")
            .field("filename", &self.filename)
            .field("offset", &self.info.offset)
            .field("size", &self.info.size)
            .finish()
    }
}

#[derive(Debug)]
pub struct DirectoryIter<'a, S: ReadableStorage> {
    fs: &'a PartitionFileSystem<S>,
    iter: std::collections::hash_map::Iter<'a, String, FileInfo>,
}

impl<'a, S: ReadableStorage> Iterator for DirectoryIter<'a, S> {
    type Item = Entry<File<'a, S>, Directory<'a, S>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(filename, &info)| {
            Entry::File(File {
                fs: self.fs,
                filename: filename.as_str(),
                info,
            })
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

        let header_size = io.stream_position().unwrap();

        let storage = io.into_inner().into_inner().shared();
        Ok(Self {
            storage,
            files,
            header_size,
        })
    }
}

impl<S: ReadableStorage> ReadableFileSystem for PartitionFileSystem<S> {
    type File<'a> = File<'a, S> where Self: 'a;
    type Directory<'a> = Directory<'a, S> where Self: 'a;

    fn root(&self) -> Self::Directory<'_> {
        Directory { fs: self }
    }

    fn open_directory(&self, path: &str) -> Option<Self::Directory<'_>> {
        assert!(path.starts_with('/'));
        if path == "/" {
            Some(self.root())
        } else {
            None
        }
    }

    fn open_file(&self, path: &str) -> Option<Self::File<'_>> {
        let path = path.strip_prefix('/').unwrap();
        self.files
            .get_key_value(path)
            .map(|(filename, &info)| File {
                fs: self,
                filename,
                info,
            })
    }
}

impl<'a, S: ReadableStorage> ReadableDirectory for Directory<'a, S> {
    type File = File<'a, S>;
    type Iter = DirectoryIter<'a, S>;

    fn name(&self) -> &str {
        ""
    }

    fn entries(&self) -> Self::Iter {
        DirectoryIter {
            fs: self.fs,
            iter: self.fs.files.iter(),
        }
    }
}

impl<'a, S: ReadableStorage> ReadableFile for File<'a, S> {
    type Storage = FileStorage<S>;
    type Error = PfsOpenError;

    fn name(&self) -> &str {
        self.filename
    }

    fn size(&self) -> u64 {
        self.info.size
    }

    fn storage(&self) -> Result<Self::Storage, Self::Error> {
        let storage = self.fs.storage.clone();
        let offset = self.info.offset + self.fs.header_size;
        let size = self.info.size;
        storage.slice(offset, size).context(PfsOpenSnafu)
    }
}
