use crate::filesystem::{Entry, ReadableDirectory, ReadableFile, ReadableFileSystem};
use indexmap::IndexMap;

pub struct MergeDirectory<'a, F: ReadableFileSystem + 'a> {
    name: String,
    directories: Vec<F::Directory<'a>>,
}

pub struct MergeFilesystem<F: ReadableFileSystem> {
    filesystems: Vec<F>,
}

pub struct MergeDirectoryIter<'a, F: ReadableFileSystem + 'a> {
    entries: Vec<Entry<F::File<'a>, MergeDirectory<'a, F>>>,
}

impl<'a, F: ReadableFileSystem + 'a> Iterator for MergeDirectoryIter<'a, F> {
    type Item = Entry<F::File<'a>, MergeDirectory<'a, F>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.pop()
    }
}

impl<'a, F: ReadableFileSystem + 'a> ReadableDirectory for MergeDirectory<'a, F> {
    type File = F::File<'a>;
    type Iter = MergeDirectoryIter<'a, F>;

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn entries(&self) -> Self::Iter {
        // we handle multiple same-named files as first come first serve basis
        // the directories are merged recursively
        // unfortunately, iteration requires allocation because we can't know in advance which iterator will have which entry

        let mut files = IndexMap::new();
        let mut directories = IndexMap::new();

        for dir in &self.directories {
            for entry in dir.entries() {
                match entry {
                    Entry::File(f) => {
                        let f: F::File<'a> = f;
                        // I think we can do less clones here by prolonging the lifes of the iterators, but meh
                        files.entry(f.name().to_string()).or_insert(f);
                    }
                    Entry::Directory(d) => {
                        let d: F::Directory<'a> = d;
                        directories
                            .entry(d.name().to_string())
                            .or_insert_with(|| MergeDirectory::<'a, F> {
                                name: d.name().to_string(),
                                directories: vec![],
                            })
                            .directories
                            .push(d);
                    }
                }
            }
        }

        let entries = directories
            .into_values()
            .map(Entry::Directory)
            .chain(files.into_values().map(Entry::File))
            .collect();

        MergeDirectoryIter { entries }
    }
}

impl<F: ReadableFileSystem> ReadableFileSystem for MergeFilesystem<F> {
    type File<'a> = F::File<'a> where Self: 'a;
    type Directory<'a> = MergeDirectory<'a, F> where Self: 'a;
    type Storage = F::Storage;
    type OpenError = F::OpenError;

    fn root(&self) -> Self::Directory<'_> {
        MergeDirectory {
            name: "".to_string(),
            directories: self
                .filesystems
                .iter()
                .map(|fs| fs.root())
                .collect::<Vec<_>>(),
        }
    }

    fn open_directory(&self, path: &str) -> Option<Self::Directory<'_>> {
        let res = self
            .filesystems
            .iter()
            .filter_map(|fs| fs.open_directory(path))
            .collect::<Vec<_>>();
        // this is indeed a manual map, but borrow checker doesn't like it otherwise
        #[allow(clippy::manual_map)]
        if let Some(first) = res.first() {
            Some(MergeDirectory {
                name: first.name().to_string(),
                directories: res,
            })
        } else {
            None
        }
    }

    fn open_file(&self, path: &str) -> Option<Self::File<'_>> {
        self.filesystems.iter().find_map(|fs| fs.open_file(path))
    }
}

impl<F: ReadableFileSystem> MergeFilesystem<F> {
    pub fn new(filesystems: Vec<F>) -> Self {
        Self { filesystems }
    }
}
