use crate::filesystem::{ReadableDirectoryExt, ReadableFile, ReadableFileSystem};
use crate::formats::cnmt::{ContentMetaType, NcmContentType, PackagedContentMeta};
use crate::formats::nacp::Nacp;
use crate::formats::nca::filesystem::NcaOpenError;
use crate::formats::nca::{IntegrityCheckLevel, Nca, NcaContentType, NcaSectionType};
use crate::ids::{AnyId, ContentId};
use crate::storage::{ReadableStorage, ReadableStorageExt, StorageError};
use crate::switch_fs::nca_set::NcaSet;
use binrw::BinRead;
use indexmap::IndexMap;
use itertools::Itertools;
use snafu::{OptionExt, ResultExt, Snafu};
use tracing::info;

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

    #[snafu(display("NCA {nca_id} mentioned in the metadata not found"))]
    MissingNca { nca_id: ContentId },
    #[snafu(display("Could not determine the main NCA for the title"))]
    MissingMainNca {},
    #[snafu(display("Could not determine the control NCA for the title"))]
    MissingControlNca {},
    #[snafu(display("Could not parse the Control NCA {control_nca_id} for the title"))]
    ControlParse {
        control_nca_id: ContentId,
        source: ControlParseError,
    },
}

#[derive(Snafu, Debug)]
#[snafu(display("Failed to parse title for meta nca {meta_nca_id}"))]
pub struct TitleSetParseError {
    pub meta_nca_id: ContentId,
    pub source: TitleParseError,
}

#[derive(Debug)]
pub struct Title {
    pub metadata: PackagedContentMeta,
    pub control: Nacp,
    pub nca_ids: Vec<ContentId>,
    pub meta_nca_id: ContentId,
    pub main_nca_id: ContentId,
    pub control_nca_id: ContentId,
}

impl Title {
    pub fn any_title(&self) -> Option<&crate::formats::nacp::ApplicationTitle> {
        self.control.any_title()
    }

    pub fn title_id(&self) -> AnyId {
        self.metadata.title_id
    }

    pub fn version(&self) -> u32 {
        self.metadata.version
    }

    pub fn ty(&self) -> ContentMetaType {
        self.metadata.ty
    }
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
    meta_nca_id: ContentId,
    meta_nca: &Nca<S>,
    nca_set: &NcaSet<S>,
) -> Result<Title, TitleParseError> {
    let fs = meta_nca
        .get_fs(NcaSectionType::Data, IntegrityCheckLevel::Full)
        .context(MetaNoDataSectionSnafu)?;
    // find the cnmt file (its name changes, but always ends with .cnmt)
    let meta = fs
        .root()
        .entries_recursive()
        .filter(|(n, _)| n.ends_with(".cnmt"))
        .filter_map(|(_, e)| e.file())
        .exactly_one()
        .map_err(|e| match e.size_hint().1.unwrap() {
            0 => TitleParseError::MetaNoCnmt {},
            _ => TitleParseError::MetaMultipleCnmt {},
        })?;
    // read the cnmt file
    let meta = meta
        .storage()
        .context(MetaCnmtOpenSnafu)?
        .read_all()
        .context(MetaCnmtReadSnafu)?;
    // and parse it!
    let meta =
        PackagedContentMeta::read(&mut std::io::Cursor::new(meta)).context(MetaCnmtParseSnafu)?;

    #[allow(clippy::match_single_binding)]
    let nca_ids: Vec<_> = match meta.ty {
        // patches list ALL the ncas in the meta_tables, including the base game and previous updates
        // we don't want that
        // UPD: Hmmm, it seems that not all patches are created equal. Some do not have much in terms of extended data at all...
        // ContentMetaType::Patch => cnmt
        //     .meta_tables
        //     .extended_data
        //     .as_ref()
        //     .unwrap()
        //     .fragment_sets
        //     .iter()
        //     .map(|v| v.nca_id_new)
        //     .collect(),
        _ => meta
            .content_info
            .iter()
            .filter(|ci| ci.content_info.ty != NcmContentType::DeltaFragment)
            .map(|v| v.content_info.content_id)
            .collect(),
    };

    // now we know the other NCAs used by the title, try to look them up
    let ncas = nca_ids
        .into_iter()
        .map(|nca_id| {
            Ok((
                nca_id,
                // note: here we ignore the missing NCAs
                // this allows us to work with some patches that ¿list too much NCAs?
                nca_set.get(&nca_id).context(MissingNcaSnafu { nca_id })?,
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // now identify the main and control NCAs by their content type
    let main_nca_id = ncas
        .iter()
        .find(|(_, n)| {
            matches!(
                n.content_type(),
                NcaContentType::Program | NcaContentType::Data
            )
        })
        .map(|(id, _)| id)
        .copied()
        .context(MissingMainNcaSnafu)?;

    let (control_nca_id, control_nca) = *ncas
        .iter()
        .find(|(_, n)| n.content_type() == NcaContentType::Control)
        .context(MissingControlNcaSnafu)?;

    let control = read_control(control_nca).context(ControlParseSnafu { control_nca_id })?;

    Ok(Title {
        metadata: meta,
        control,
        nca_ids: ncas.into_iter().map(|(id, _)| id).collect(),
        meta_nca_id,
        main_nca_id,
        control_nca_id,
    })
}

// Key is a pair of (TitleId, Version) to allow multiple versions of the same title
// TODO: use a separate type for Version
pub type TitleSet = IndexMap<(AnyId, u32), Title>;

pub fn title_set_from_nca_set<S: ReadableStorage>(
    ncas: &NcaSet<S>,
) -> Result<TitleSet, TitleSetParseError> {
    let mut titles = IndexMap::new();

    for (&id, nca) in ncas {
        if nca.content_type() == NcaContentType::Meta {
            info!("Parsing title for meta nca {}", id);
            let title =
                parse_title(id, nca, ncas).context(TitleSetParseSnafu { meta_nca_id: id })?;
            titles.insert((title.title_id(), title.version()), title);
        }
    }

    Ok(titles)
}
