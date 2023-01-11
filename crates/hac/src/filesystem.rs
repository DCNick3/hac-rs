use crate::storage::ReadableStorage;
use std::fmt::Debug;

#[derive(Debug)]
pub enum Entry<F: ReadableFile, D: ReadableDirectory> {
    File(F),
    Directory(D),
}

pub trait ReadableFile: Sized {
    type Storage: ReadableStorage;
    type Error: Debug;

    fn name(&self) -> &str;
    fn size(&self) -> u64;
    fn storage(&self) -> Result<Self::Storage, Self::Error>;
}

pub trait ReadableDirectory: Sized {
    type File: ReadableFile;
    type Iter: Iterator<Item = Entry<Self::File, Self>>;

    fn name(&self) -> &str;
    fn entries(&self) -> Self::Iter;
}

pub trait ReadableFileSystem: Sized {
    type File<'a>: ReadableFile + 'a
    where
        Self: 'a;
    type Directory<'a>: ReadableDirectory<File = Self::File<'a>>
    where
        Self: 'a;

    fn root(&self) -> Self::Directory<'_>;
    fn open_directory(&self, path: &str) -> Option<Self::Directory<'_>>;
    fn open_file(&self, path: &str) -> Option<Self::File<'_>>;
}
