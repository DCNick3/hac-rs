use crate::crypto::keyset::KeySet;
use crate::filesystem::{Entry, ReadableDirectoryExt, ReadableFile, ReadableFileSystem};
use crate::formats::cnmt::Cnmt;
use crate::formats::nca::{IntegrityCheckLevel, Nca, NcaContentType, NcaSectionType};
use crate::storage::{ReadableStorage, ReadableStorageExt};
use crate::types::{NcaId, TitleId};
use binrw::BinRead;
use itertools::Itertools;
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::fmt::Debug;
use tracing::warn;

#[derive(Snafu, Debug)]
pub enum NcasFromFsError {
    NcaParse {
        nca_id: NcaId,
        source: crate::formats::nca::NcaError,
    },
}

struct Ncas<S: ReadableStorage>(HashMap<NcaId, Nca<S>>);

fn get_nca_id(filename: &str) -> Option<NcaId> {
    let filename = filename
        .strip_suffix(".cnmt.nca")
        .or_else(|| filename.strip_suffix(".nca"))?;

    filename.parse().ok()
}

impl<S: ReadableStorage> Ncas<S> {
    pub fn from_fs<F: ReadableFileSystem<Storage = S>>(
        key_set: &KeySet,
        fs: &F,
    ) -> Result<Self, NcasFromFsError> {
        let mut ncas = HashMap::new();

        for file in ReadableDirectoryExt::entries_recursive(&fs.root())
            .filter(|(n, _)| n.ends_with(".nca"))
            .filter_map(|(_, e)| e.file())
        {
            let storage = file.storage().expect("Malformed FS");
            if let Some(nca_id) = get_nca_id(file.name()) {
                let nca = Nca::new(key_set, storage).context(NcaParseSnafu { nca_id })?;
                ncas.insert(nca_id, nca);
            } else {
                warn!("Invalid NCA filename: {}", file.name());
            }
        }

        Ok(Self(ncas))
    }
}

#[derive(Debug)]
pub struct Title(Cnmt);

#[derive(Debug)]
struct Titles(HashMap<TitleId, Title>);

impl Titles {
    pub fn from_ncas<S: ReadableStorage>(ncas: &Ncas<S>) -> Self {
        let mut titles = HashMap::new();

        for (id, nca) in &ncas.0 {
            if nca.content_type() == NcaContentType::Meta {
                if let Some(fs) = nca.get_fs(NcaSectionType::Data, IntegrityCheckLevel::Full) {
                    let cnmt = fs
                        .root()
                        .entries_recursive()
                        .filter(|(n, _)| n.ends_with(".cnmt"))
                        .filter_map(|(_, e)| e.file())
                        .exactly_one()
                        .unwrap_or_else(|_| panic!("Meta NCA has CNMT count != 1: {:?}", id));
                    let cnmt = cnmt.storage().expect("Malformed NCA").read_all().unwrap();
                    let cnmt = Cnmt::read(&mut std::io::Cursor::new(cnmt)).expect("Malformed CNMT");

                    let title_id = cnmt.title_id;
                    titles.insert(title_id, Title(cnmt));
                } else {
                    warn!(
                        "NCA {:?} is missing data section, even though it's a Meta NCA",
                        id
                    );
                }
            }
        }

        Self(titles)
    }
}

pub struct SwitchFs<F: ReadableFileSystem> {
    ncas: Ncas<F::Storage>,
}

impl<F: ReadableFileSystem> SwitchFs<F> {
    pub fn new(key_set: &KeySet, fs: &F) -> Result<Self, NcasFromFsError> {
        let key_set = key_set.clone();

        // TODO: import tickets from the FS

        let ncas = Ncas::from_fs(&key_set, fs)?;
        dbg!(ncas.0.keys().collect::<Vec<_>>());
        let titles = Titles::from_ncas(&ncas);

        dbg!(titles);

        Ok(Self { ncas })
    }
}
