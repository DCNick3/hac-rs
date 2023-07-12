mod program;

use crate::filesystem::{ReadableDirectoryExt, ReadableFile, ReadableFileSystem};
use crate::formats::cnmt::{
    ContentMetaKey, ContentMetaType, ExtendedMetaHeader, NcmContentType, PackagedContentMeta,
};
use crate::formats::nacp::{ApplicationControlProperty, ProgramTitle};
use crate::formats::nca::filesystem::NcaOpenError;
use crate::formats::nca::{IntegrityCheckLevel, Nca, NcaContentType, NcaSectionType};
use crate::ids::{ApplicationId, ContentId, DataId, DataPatchId, PatchId, ProgramId};
use crate::storage::{ReadableStorage, ReadableStorageExt, StorageError};
use crate::switch_fs::content_set::program::ProgramsParseError;
use crate::switch_fs::nca_set::NcaSet;
use binrw::BinRead;
use itertools::Itertools;
use snafu::{OptionExt, ResultExt, Snafu};
use std::collections::BTreeMap;
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
pub enum ContentParseError {
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

    #[snafu(display("Unsupported meta type {ty:?}"))]
    MetaUnsupportedType { ty: ContentMetaType },

    #[snafu(display("Failed to parse the programs for the Content"))]
    ProgramsParse { source: ProgramsParseError },

    #[snafu(display("NCA {nca_id} mentioned in the metadata not found"))]
    MissingNca { nca_id: ContentId },
    #[snafu(display("Could not find the main NCA for the title"))]
    MissingMainNca {},
    #[snafu(display("Could not find the control NCA for the title"))]
    MissingControlNca {},
    #[snafu(display("Could not find the legal information NCA for the title"))]
    MissingLegalInformationNca {},
    #[snafu(display("Could not find the data NCA for the title"))]
    MissingDataNca {},
    #[snafu(display("Could not parse the Control NCA {control_nca_id} for the title"))]
    ControlParse {
        control_nca_id: ContentId,
        source: ControlParseError,
    },
}

#[derive(Snafu, Debug)]
#[snafu(display("Failed to parse title for meta nca {meta_nca_id}"))]
pub struct ContentSetParseError {
    pub meta_nca_id: ContentId,
    pub source: ContentParseError,
}

#[derive(Debug)]
pub struct ContentInfoCommon {
    pub metadata: PackagedContentMeta,
    pub contents: Vec<ContentId>,
    pub meta_content_id: ContentId,
}

impl ContentInfoCommon {
    pub fn content_meta_key(&self) -> ContentMetaKey {
        self.metadata.content_meta_key()
    }
}

#[derive(Debug)]
pub struct ProgramInfo {
    pub id: ProgramId,
    // only set for programs in the patch
    pub base_program_id: Option<ProgramId>,
    pub program_content_id: ContentId,
    pub control_content_id: ContentId,
    pub html_document_content_id: Option<ContentId>,
    pub control: ApplicationControlProperty,
}

/// Corresponds to [`ContentMetaType::Application`]
#[derive(Debug)]
pub struct ApplicationInfo {
    pub id: ApplicationId,
    pub patch_id: PatchId,
    pub legal_information_content: ContentId,
    pub programs: Vec<ProgramInfo>,
    pub common: ContentInfoCommon,
}

impl ApplicationInfo {
    pub fn any_title(&self) -> Option<&ProgramTitle> {
        self.programs.iter().find_map(|p| p.control.any_title())
    }
}

/// Corresponds to [`ContentMetaType::Patch`]
#[derive(Debug)]
pub struct PatchInfo {
    pub id: PatchId,
    pub application_id: ApplicationId,
    pub legal_information_content: ContentId,
    pub programs: Vec<ProgramInfo>,
    pub common: ContentInfoCommon,
}

impl PatchInfo {
    pub fn any_title(&self) -> Option<&ProgramTitle> {
        self.programs.iter().find_map(|p| p.control.any_title())
    }
}

/// Corresponds to [`ContentMetaType::AddOnContent`]
#[derive(Debug)]
pub struct DataInfo {
    pub id: DataId,
    pub application_id: ApplicationId,
    pub data_patch_id: DataPatchId,
    pub data_content: ContentId,
    pub common: ContentInfoCommon,
}

/// Corresponds to [`ContentMetaType::DataPatch`]
#[derive(Debug)]
pub struct DataPatchInfo {
    pub id: DataPatchId,
    pub common: ContentInfoCommon,
}

#[derive(Debug)]
pub enum AnyContentInfo {
    Application(ApplicationInfo),
    Patch(PatchInfo),
    Data(DataInfo),
    DataPatch(DataPatchInfo),
}

impl AnyContentInfo {
    pub fn common_info(&self) -> &ContentInfoCommon {
        match self {
            AnyContentInfo::Application(info) => &info.common,
            AnyContentInfo::Patch(info) => &info.common,
            AnyContentInfo::Data(info) => &info.common,
            AnyContentInfo::DataPatch(info) => &info.common,
        }
    }

    pub fn content_meta_key(&self) -> ContentMetaKey {
        self.common_info().content_meta_key()
    }
}

fn find_content_of_type(meta: &PackagedContentMeta, ty: NcmContentType) -> Option<ContentId> {
    meta.content_info
        .iter()
        .find(|ci| ci.content_info.ty == ty)
        .map(|ci| ci.content_info.id)
}

fn parse_content<S: ReadableStorage>(
    meta_content_id: ContentId,
    meta_nca: &Nca<S>,
    nca_set: &NcaSet<S>,
) -> Result<AnyContentInfo, ContentParseError> {
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
            0 => ContentParseError::MetaNoCnmt {},
            _ => ContentParseError::MetaMultipleCnmt {},
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

    // dbg!(&meta);

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
            .map(|v| v.content_info.id)
            .collect(),
    };

    // now we know the other NCAs used by the title, try to look them up
    let contents = nca_ids
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

    let common = ContentInfoCommon {
        metadata: meta.clone(),
        contents: contents.iter().map(|&(id, _)| id).collect(),
        meta_content_id,
    };

    Ok(match meta.ty {
        ContentMetaType::SystemProgram
        | ContentMetaType::SystemData
        | ContentMetaType::SystemUpdate
        | ContentMetaType::BootImagePackage
        | ContentMetaType::BootImagePackageSafe
        | ContentMetaType::Delta => {
            // these are not supported (at least yet)
            return Err(ContentParseError::MetaUnsupportedType { ty: meta.ty });
        }

        ContentMetaType::Application => {
            let ExtendedMetaHeader::Application { patch_id, .. } = meta.extended_header else {
                unreachable!()
            };

            let programs = program::parse_programs(&meta, nca_set).context(ProgramsParseSnafu)?;
            let legal_information_content =
                find_content_of_type(&meta, NcmContentType::LegalInformation)
                    .context(MissingLegalInformationNcaSnafu)?;

            AnyContentInfo::Application(ApplicationInfo {
                id: meta.id.into(),
                patch_id,
                legal_information_content,
                programs,
                common,
            })
        }
        ContentMetaType::Patch => {
            let ExtendedMetaHeader::Patch { application_id, .. } = meta.extended_header else {
                unreachable!()
            };

            let programs = program::parse_programs(&meta, nca_set).context(ProgramsParseSnafu)?;
            let legal_information_content =
                find_content_of_type(&meta, NcmContentType::LegalInformation)
                    .context(MissingLegalInformationNcaSnafu)?;

            AnyContentInfo::Patch(PatchInfo {
                id: meta.id.into(),
                application_id,
                legal_information_content,
                programs,
                common,
            })
        }
        ContentMetaType::AddOnContent => {
            let ExtendedMetaHeader::AddOnContent { application_id, data_patch_id, .. } = meta.extended_header else {
                unreachable!()
            };

            let data_content =
                find_content_of_type(&meta, NcmContentType::Data).context(MissingDataNcaSnafu)?;

            AnyContentInfo::Data(DataInfo {
                id: meta.id.into(),
                application_id,
                data_patch_id,
                data_content,
                common,
            })
        }
        ContentMetaType::DataPatch => todo!("Handling of DataPatch is not implemented yet"),
    })

    // now identify the main and control NCAs by their content type
    // let main_nca_id = contents
    //     .iter()
    //     .find(|(_, n)| {
    //         matches!(
    //             n.content_type(),
    //             NcaContentType::Program | NcaContentType::Data
    //         )
    //     })
    //     .map(|(id, _)| id)
    //     .copied()
    //     .context(MissingMainNcaSnafu)?;
    //
    // let (control_nca_id, control_nca) = *contents
    //     .iter()
    //     .find(|(_, n)| n.content_type() == NcaContentType::Control)
    //     .context(MissingControlNcaSnafu)?;
    //
    // let control = read_control(control_nca).context(ControlParseSnafu { control_nca_id })?;
}

pub type ContentSet = BTreeMap<ContentMetaKey, AnyContentInfo>;

pub fn content_set_from_nca_set<S: ReadableStorage>(
    ncas: &NcaSet<S>,
) -> Result<ContentSet, ContentSetParseError> {
    let mut titles = BTreeMap::new();

    for (&id, nca) in ncas {
        if nca.content_type() == NcaContentType::Meta {
            info!("Parsing title for meta nca {}", id);
            let content =
                parse_content(id, nca, ncas).context(ContentSetParseSnafu { meta_nca_id: id })?;

            // dbg!(&content);

            titles.insert(content.content_meta_key(), content);
        }
    }

    Ok(titles)
}
