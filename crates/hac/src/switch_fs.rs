use crate::crypto::keyset::KeySet;
use crate::filesystem::{ReadableDirectoryExt, ReadableFile, ReadableFileSystem};
use crate::formats::cnmt::Cnmt;
use crate::formats::nacp::Nacp;
use crate::formats::nca::{IntegrityCheckLevel, Nca, NcaContentType, NcaSectionType};
use crate::storage::{ReadableStorage, ReadableStorageExt};
use crate::types::{NcaId, TitleId};
use binrw::BinRead;
use itertools::Itertools;
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tracing::warn;

#[derive(Snafu, Debug)]
pub enum NcasFromFsError {
    NcaParse {
        nca_id: NcaId,
        source: crate::formats::nca::NcaError,
    },
}

#[derive(Debug)]
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
pub struct Title {
    metadata: Cnmt,
    control: Nacp,
    ncas: Vec<NcaId>,
    main_nca_id: NcaId,
    control_nca_id: NcaId,
}

fn read_control<S: ReadableStorage>(nca: &Nca<S>) -> Nacp {
    let fs = nca
        .get_fs(NcaSectionType::Data, IntegrityCheckLevel::Full)
        .expect("TODO: remove the panic");

    dbg!(fs
        .root()
        .entries_recursive()
        .map(|v| v.0)
        .collect::<Vec<_>>());

    let file = fs
        .open_file("/control.nacp")
        .expect("TODO: remove the panic");
    let control = file
        .storage()
        .expect("TODO: remove the panic")
        .read_all()
        .expect("TODO: remove the panic");
    Nacp::read(&mut std::io::Cursor::new(control)).expect("TODO: remove the panic")
}

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
                    let ncas = cnmt
                        .meta_tables
                        .content_entries
                        .iter()
                        .map(|e| (e.nca_id, ncas.0.get(&e.nca_id).expect("Missing NCA")))
                        .collect::<Vec<_>>();

                    let main_nca_id = *ncas
                        .iter()
                        .find(|(_, n)| {
                            matches!(
                                n.content_type(),
                                NcaContentType::Program | NcaContentType::Data
                            )
                        })
                        .map(|(id, _)| id)
                        .expect("Missing main NCA");
                    let (control_nca_id, control_nca) = *ncas
                        .iter()
                        .find(|(_, n)| n.content_type() == NcaContentType::Control)
                        .expect("Missing control NCA");

                    let control = read_control(control_nca);

                    titles.insert(
                        title_id,
                        Title {
                            metadata: cnmt,
                            control,
                            ncas: ncas.into_iter().map(|(id, _)| id).collect(),
                            main_nca_id,
                            control_nca_id,
                        },
                    );
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

#[derive(Debug)]
pub struct SwitchFs<F: ReadableFileSystem> {
    ncas: Ncas<F::Storage>,
    titles: Titles,
}

impl<F: ReadableFileSystem> SwitchFs<F> {
    pub fn new(key_set: &KeySet, fs: &F) -> Result<Self, NcasFromFsError> {
        let key_set = key_set.clone();

        // TODO: import tickets from the FS

        let ncas = Ncas::from_fs(&key_set, fs)?;
        let titles = Titles::from_ncas(&ncas);

        Ok(Self { ncas, titles })
    }
}
