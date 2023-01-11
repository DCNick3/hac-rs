use crate::formats::romfs::structs::{RomFsEntry, RomId};
use crate::storage::{ReadableStorage, ReadableStorageExt};
use binrw::{BinRead, BinWrite};
use std::marker::PhantomData;

#[derive(BinRead)]
#[br(little)]
struct Buckets(#[br(parse_with = binrw::until_eof)] Vec<RomId>);

#[derive(BinRead)]
#[br(little)]
struct Entries(#[br(parse_with = binrw::until_eof)] Vec<u8>);

#[derive(Debug)]
pub struct RomFsDictionary<T: BinRead<Args = ()> + BinWrite<Args = ()>> {
    buckets: Vec<RomId>,
    entries: Vec<u8>,
    phantom: PhantomData<T>,
}

impl<T: BinRead<Args = ()> + BinWrite<Args = ()>> RomFsDictionary<T> {
    pub fn new(buckets: Vec<RomId>, entries: Vec<u8>) -> Self {
        Self {
            buckets,
            entries,
            phantom: PhantomData,
        }
    }

    pub fn from_storage(
        buckets: impl ReadableStorage,
        entries: impl ReadableStorage,
    ) -> Result<Self, binrw::Error> {
        let buckets = Buckets::read(&mut buckets.buf_read())?.0;
        let entries = Entries::read(&mut entries.buf_read())?.0;

        Ok(Self::new(buckets, entries))
    }

    pub fn get_offset_from_key(&self, key: RomEntryKey) -> Option<(&str, RomId)> {
        let hash = key.hash();
        let index = hash as usize % self.buckets.len();
        let mut id = self.buckets[index];

        while id.is_some() {
            let (name, entry) = self.get_entry_by_id(id);

            if entry.parent == key.parent && name == key.name {
                return Some((name, id));
            }

            id = entry.next;
        }

        None
    }

    pub fn get_entry_by_key(&self, key: RomEntryKey) -> Option<(&str, RomFsEntry<T>)> {
        let (_name, id) = self.get_offset_from_key(key)?;

        Some(self.get_entry_by_id(id))
    }

    pub fn get_entry_by_id(&self, id: RomId) -> (&str, RomFsEntry<T>) {
        assert!(id.is_some());

        let mut cur = std::io::Cursor::new(&self.entries);
        cur.set_position(id.0 as u64);

        let entry = RomFsEntry::read(&mut cur).unwrap();

        let key = &self.entries[cur.position() as usize..][..entry.key_length as usize];

        let key = std::str::from_utf8(key).expect("Invalid UTF-8 in RomFS dictionary");

        (key, entry)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RomEntryKey<'a> {
    pub name: &'a str,
    pub parent: RomId,
}

impl<'a> RomEntryKey<'a> {
    pub fn hash(&self) -> u32 {
        let mut hash = 123456789 ^ self.parent.0 as u32;

        for c in self.name.bytes() {
            hash = (c as u32) ^ ((hash << 27) | (hash >> 5));
        }

        hash
    }
}
