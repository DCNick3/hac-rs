use crate::storage::ReadableStorage;
use snafu::AsErrorSource;
use std::fmt::{Debug, Display};

#[derive(Debug)]
pub enum Entry<F: ReadableFile, D: ReadableDirectory> {
    File(F),
    Directory(D),
}

impl<F: ReadableFile, D: ReadableDirectory> Entry<F, D> {
    pub fn file(self) -> Option<F> {
        match self {
            Entry::File(f) => Some(f),
            _ => None,
        }
    }

    pub fn directory(self) -> Option<D> {
        match self {
            Entry::Directory(d) => Some(d),
            _ => None,
        }
    }
}

pub trait ReadableFile: Sized {
    type Storage: ReadableStorage;
    type Error: Debug + Display;

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
    type File<'a>: ReadableFile<Storage = Self::Storage, Error = Self::OpenError> + 'a
    where
        Self: 'a;
    type Directory<'a>: ReadableDirectory<File = Self::File<'a>>
    where
        Self: 'a;
    type Storage: ReadableStorage;
    type OpenError: Debug + Display + AsErrorSource;

    fn root(&self) -> Self::Directory<'_>;
    fn open_directory(&self, path: &str) -> Option<Self::Directory<'_>>;
    fn open_file(&self, path: &str) -> Option<Self::File<'_>>;
}

pub struct RecursiveDirectoryIter<D: ReadableDirectory> {
    inner: Vec<D::Iter>,
    path: String,
}

impl<D: ReadableDirectory> Iterator for RecursiveDirectoryIter<D> {
    type Item = (String, Entry<D::File, D>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(it) = self.inner.last_mut() {
                match it.next() {
                    None => {
                        self.inner.pop().unwrap();
                    }
                    Some(Entry::File(f)) => {
                        break Some((format!("{}/{}", self.path, f.name()), Entry::File(f)))
                    }
                    Some(Entry::Directory(d)) => {
                        self.inner.push(d.entries());
                        self.path.push('/');
                        self.path.push_str(d.name());
                        break Some((self.path.clone(), Entry::Directory(d)));
                    }
                    #[allow(dead_code, unreachable_patterns)]
                    // Clion doesn't understand that this is unreachable and wants me to "cover all match args"
                    _ => unreachable!(),
                }
            } else {
                break None;
            }
        }
    }
}

pub trait ReadableDirectoryExt: ReadableDirectory {
    fn entries_recursive(&self) -> RecursiveDirectoryIter<Self> {
        RecursiveDirectoryIter {
            inner: vec![self.entries()],
            path: "".to_string(),
        }
    }
}

impl<T: ReadableDirectory> ReadableDirectoryExt for T {}
