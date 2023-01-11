use crate::filesystem::{Entry, ReadableDirectory, ReadableFile, ReadableFileSystem};
use crate::formats::pfs::{PartitionFileSystem, PfsParseError};
use crate::formats::romfs::{RomFileSystem, RomFsParseError};
use crate::formats::{pfs, romfs};
use crate::storage::ReadableStorage;

type NcaFileStorage<S> = pfs::FileStorage<S>;

#[derive(Debug)]
pub enum NcaFileSystem<S: ReadableStorage> {
    Romfs(RomFileSystem<S>),
    Pfs(PartitionFileSystem<S>),
}

#[derive(Debug)]
pub enum NcaFile<'a, S: ReadableStorage> {
    Romfs(romfs::File<'a, S>),
    Pfs(pfs::File<'a, S>),
}

#[derive(Debug)]
pub enum NcaDirectory<'a, S: ReadableStorage> {
    Romfs(romfs::Directory<'a, S>),
    Pfs(pfs::Directory<'a, S>),
}

#[derive(Debug)]
pub enum NcaDirectoryIter<'a, S: ReadableStorage> {
    Romfs(romfs::DirectoryIter<'a, S>),
    Pfs(pfs::DirectoryIter<'a, S>),
}

#[derive(Debug)]
pub enum NcaOpenError {
    Romfs(romfs::RomfsOpenError),
    Pfs(pfs::PfsOpenError),
}

impl<S: ReadableStorage> NcaFileSystem<S> {
    pub fn new_romfs(storage: S) -> Result<Self, RomFsParseError> {
        Ok(Self::Romfs(RomFileSystem::new(storage)?))
    }

    pub fn new_pfs(storage: S) -> Result<Self, PfsParseError> {
        Ok(Self::Pfs(PartitionFileSystem::new(storage)?))
    }
}

impl<S: ReadableStorage> ReadableFileSystem for NcaFileSystem<S> {
    type File<'a> = NcaFile<'a, S> where Self: 'a;
    type Directory<'a> = NcaDirectory<'a, S> where Self: 'a;

    fn root(&self) -> Self::Directory<'_> {
        match self {
            NcaFileSystem::Romfs(fs) => NcaDirectory::Romfs(fs.root()),
            NcaFileSystem::Pfs(fs) => NcaDirectory::Pfs(fs.root()),
        }
    }

    fn open_directory(&self, path: &str) -> Option<Self::Directory<'_>> {
        match self {
            NcaFileSystem::Romfs(fs) => fs.open_directory(path).map(NcaDirectory::Romfs),
            NcaFileSystem::Pfs(fs) => fs.open_directory(path).map(NcaDirectory::Pfs),
        }
    }

    fn open_file(&self, path: &str) -> Option<Self::File<'_>> {
        match self {
            NcaFileSystem::Romfs(fs) => fs.open_file(path).map(NcaFile::Romfs),
            NcaFileSystem::Pfs(fs) => fs.open_file(path).map(NcaFile::Pfs),
        }
    }
}

impl<'a, S: ReadableStorage> ReadableFile for NcaFile<'a, S> {
    type Storage = NcaFileStorage<S>;
    type Error = NcaOpenError;

    fn name(&self) -> &str {
        match self {
            NcaFile::Romfs(file) => file.name(),
            NcaFile::Pfs(file) => file.name(),
        }
    }

    fn size(&self) -> u64 {
        match self {
            NcaFile::Romfs(file) => file.size(),
            NcaFile::Pfs(file) => file.size(),
        }
    }

    fn storage(&self) -> Result<Self::Storage, Self::Error> {
        match self {
            NcaFile::Romfs(file) => file.storage().map_err(NcaOpenError::Romfs),
            NcaFile::Pfs(file) => file.storage().map_err(NcaOpenError::Pfs),
        }
    }
}

impl<'a, S: ReadableStorage> ReadableDirectory for NcaDirectory<'a, S> {
    type File = NcaFile<'a, S>;
    type Iter = NcaDirectoryIter<'a, S>;

    fn name(&self) -> &str {
        match self {
            NcaDirectory::Romfs(dir) => dir.name(),
            NcaDirectory::Pfs(dir) => dir.name(),
        }
    }

    fn entries(&self) -> Self::Iter {
        match self {
            NcaDirectory::Romfs(dir) => NcaDirectoryIter::Romfs(dir.entries()),
            NcaDirectory::Pfs(dir) => NcaDirectoryIter::Pfs(dir.entries()),
        }
    }
}

impl<'a, S: ReadableStorage> Iterator for NcaDirectoryIter<'a, S> {
    type Item = Entry<NcaFile<'a, S>, NcaDirectory<'a, S>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            NcaDirectoryIter::Romfs(iter) => iter.next().map(|entry| match entry {
                Entry::File(file) => Entry::File(NcaFile::Romfs(file)),
                Entry::Directory(dir) => Entry::Directory(NcaDirectory::Romfs(dir)),
            }),
            NcaDirectoryIter::Pfs(iter) => iter.next().map(|entry| match entry {
                Entry::File(file) => Entry::File(NcaFile::Pfs(file)),
                Entry::Directory(dir) => Entry::Directory(NcaDirectory::Pfs(dir)),
            }),
        }
    }
}
