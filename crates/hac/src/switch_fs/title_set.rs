use crate::filesystem::{ReadableDirectoryExt, ReadableFile, ReadableFileSystem};
use crate::formats::cnmt::Cnmt;
use crate::formats::nacp::Nacp;
use crate::formats::nca::filesystem::NcaOpenError;
use crate::formats::nca::{IntegrityCheckLevel, Nca, NcaContentType, NcaSectionType};
use crate::storage::{ReadableStorage, ReadableStorageExt, StorageError};
use crate::switch_fs::nca_set::NcaSet;
use crate::types::{NcaId, TitleId};
use binrw::BinRead;
use itertools::Itertools;
use snafu::{OptionExt, ResultExt, Snafu};
use std::collections::HashMap;

#[derive(Snafu, Debug)]
pub enum ControlParseError {
    #[snafu(display("Control NCA does not have the data section"))]
    NoDataSection {},
    #[snafu(display("Control NCA does not the control.nacp file"))]
    NoControlNacp {},
    #[snafu(display("Failed to open control.nacp"))]
    ControlNacpOpen { source: NcaOpenError },
    #[snafu(display("Failed to read control.nacp"))]
    ControlNacpRead { source: StorageError },
    #[snafu(display("Failed to parse control.nacp"))]
    ControlNacpParse { source: binrw::Error },
}

#[derive(Snafu, Debug)]
pub enum TitleParseError {
    #[snafu(display("Meta NCA does not have the data section"))]
    MetaNoDataSection {},
    #[snafu(display("Meta NCA has multiple CNMT"))]
    MetaMultipleCnmt {},
    #[snafu(display("Meta NCA has no CNMT"))]
    MetaNoCnmt {},

    #[snafu(display("Failed to open the CNMT file"))]
    MetaCnmtOpen { source: NcaOpenError },
    #[snafu(display("Failed to read the CNMT file"))]
    MetaCnmtRead { source: StorageError },
    #[snafu(display("Failed to parse the CNMT file"))]
    MetaCnmtParse { source: binrw::Error },

    #[snafu(display("NCA mentioned in the metadata not found"))]
    MissingNca { nca_id: NcaId },
    #[snafu(display("Could not determine the main NCA for the title"))]
    MissingMainNca {},
    #[snafu(display("Could not determine the control NCA for the title"))]
    MissingControlNca {},
    #[snafu(display("Could not parse the Control NCA {control_nca_id} for the title"))]
    ControlParse {
        control_nca_id: NcaId,
        source: ControlParseError,
    },
}

#[derive(Snafu, Debug)]
#[snafu(display("Failed to parse title for meta nca {meta_nca_id}"))]
pub struct TitleSetParseError {
    pub meta_nca_id: NcaId,
    pub source: TitleParseError,
}

#[derive(Debug)]
pub struct Title {
    pub metadata: Cnmt,
    pub control: Nacp,
    pub ncas: Vec<NcaId>,
    pub meta_nca_id: NcaId,
    pub main_nca_id: NcaId,
    pub control_nca_id: NcaId,
}

fn read_control<S: ReadableStorage>(nca: &Nca<S>) -> Result<Nacp, ControlParseError> {
    let fs = nca
        .get_fs(NcaSectionType::Data, IntegrityCheckLevel::Full)
        .context(NoDataSectionSnafu)?;

    let file = fs.open_file("/control.nacp").context(NoControlNacpSnafu)?;
    let control = file
        .storage()
        .context(ControlNacpOpenSnafu)?
        .read_all()
        .context(ControlNacpReadSnafu)?;
    Nacp::read(&mut std::io::Cursor::new(control)).context(ControlNacpParseSnafu)
}

fn parse_title<S: ReadableStorage>(
    meta_nca_id: NcaId,
    meta_nca: &Nca<S>,
    nca_set: &NcaSet<S>,
) -> Result<(TitleId, Title), TitleParseError> {
    let fs = meta_nca
        .get_fs(NcaSectionType::Data, IntegrityCheckLevel::Full)
        .context(MetaNoDataSectionSnafu)?;
    let cnmt = fs
        .root()
        .entries_recursive()
        .filter(|(n, _)| n.ends_with(".cnmt"))
        .filter_map(|(_, e)| e.file())
        .exactly_one()
        .map_err(|e| match e.size_hint().1.unwrap() {
            0 => TitleParseError::MetaNoCnmt {},
            _ => TitleParseError::MetaMultipleCnmt {},
        })?;
    let cnmt = cnmt
        .storage()
        .context(MetaCnmtOpenSnafu)?
        .read_all()
        .context(MetaCnmtReadSnafu)?;
    let cnmt = Cnmt::read(&mut std::io::Cursor::new(cnmt)).context(MetaCnmtParseSnafu)?;

    let title_id = cnmt.title_id;
    let ncas = cnmt
        .meta_tables
        .content_entries
        .iter()
        .map(|e| (e.nca_id, nca_set.get(&e.nca_id).expect("Missing NCA")))
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
        .context(MissingMainNcaSnafu)?;
    let (control_nca_id, control_nca) = *ncas
        .iter()
        .find(|(_, n)| n.content_type() == NcaContentType::Control)
        .context(MissingControlNcaSnafu)?;

    let control = read_control(control_nca).context(ControlParseSnafu { control_nca_id })?;

    Ok((
        title_id,
        Title {
            metadata: cnmt,
            control,
            ncas: ncas.into_iter().map(|(id, _)| id).collect(),
            meta_nca_id,
            main_nca_id,
            control_nca_id,
        },
    ))
}

pub type TitleSet = HashMap<TitleId, Title>;

pub fn title_set_from_ncas<S: ReadableStorage>(
    ncas: &NcaSet<S>,
) -> Result<TitleSet, TitleSetParseError> {
    let mut titles = HashMap::new();

    for (&id, nca) in ncas {
        if nca.content_type() == NcaContentType::Meta {
            let (title_id, title) =
                parse_title(id, nca, ncas).context(TitleSetParseSnafu { meta_nca_id: id })?;
            titles.insert(title_id, title);
        }
    }

    Ok(titles)
}
