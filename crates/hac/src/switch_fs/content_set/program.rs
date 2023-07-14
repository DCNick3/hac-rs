use crate::filesystem::{ReadableFile, ReadableFileSystem};
use crate::formats::cnmt::{ExtendedMetaHeader, NcmContentType, PackagedContentMeta};
use crate::formats::nacp::ApplicationControlProperty;
use crate::formats::nca::{IntegrityCheckLevel, Nca, NcaSectionType};
use crate::ids::{ContentId, ProgramId};
use crate::storage::{ReadableStorage, ReadableStorageExt};
use crate::switch_fs::content_set::{
    ControlNacpOpenSnafu, ControlNacpParseSnafu, ControlNacpReadSnafu, NoControlNacpSnafu,
    NoDataSectionSnafu,
};
use crate::switch_fs::{ControlParseError, NcaSet, ProgramInfo};
use binrw::BinRead;
use snafu::{OptionExt, ResultExt, Snafu};
use std::collections::BTreeMap;

#[derive(Snafu, Debug)]
pub enum ProgramParseError {
    /// Program is missing the Program NCA
    MissingProgramContent {},
    /// Program is missing the Control NCA
    MissingControlContent {},
    /// Could not parse the Control NCA {control_content_id} for the program
    ControlParse {
        control_content_id: ContentId,
        source: ControlParseError,
    },
}

/// Could not parse one of the programs
#[derive(Snafu, Debug)]
pub struct ProgramsParseError {
    program: ProgramId,
    source: ProgramParseError,
}

fn read_control<S: ReadableStorage>(
    nca: &Nca<S>,
) -> Result<ApplicationControlProperty, ControlParseError> {
    let fs = nca
        .get_fs(NcaSectionType::Data, IntegrityCheckLevel::Full)
        .context(NoDataSectionSnafu)?;

    let file = fs.open_file("/control.nacp").context(NoControlNacpSnafu)?;
    let control = file
        .storage()
        .context(ControlNacpOpenSnafu)?
        .read_all()
        .context(ControlNacpReadSnafu)?;
    ApplicationControlProperty::read(&mut std::io::Cursor::new(control))
        .context(ControlNacpParseSnafu)
}

struct ProgramInfoBuilder {
    id: ProgramId,
    base_program_id: Option<ProgramId>,
    program_content: Option<ContentId>,
    control_content: Option<ContentId>,
    html_document_content: Option<ContentId>,
}

impl ProgramInfoBuilder {
    fn new(id: ProgramId, base_program_id: Option<ProgramId>) -> Self {
        Self {
            id,
            base_program_id,
            program_content: None,
            control_content: None,
            html_document_content: None,
        }
    }

    fn build<S: ReadableStorage>(
        self,
        nca_set: &NcaSet<S>,
    ) -> Result<ProgramInfo, ProgramParseError> {
        let program_content_id = self.program_content.context(MissingProgramContentSnafu)?;
        let control_content_id = self.control_content.context(MissingControlContentSnafu)?;
        let html_document_content_id = self.html_document_content;

        let control = nca_set.get(&control_content_id).unwrap();
        let control = read_control(control).context(ControlParseSnafu { control_content_id })?;

        Ok(ProgramInfo {
            id: self.id,
            base_program_id: self.base_program_id,
            program_content_id,
            control_content_id,
            html_document_content_id,
            control,
        })
    }
}

pub fn parse_programs<S: ReadableStorage>(
    meta: &PackagedContentMeta,
    // pre-condition: all the NCAs mentioned in the meta are in the NCA set
    nca_set: &NcaSet<S>,
) -> Result<Vec<ProgramInfo>, ProgramsParseError> {
    let id_base = meta.id;
    let base_id_base =
        if let ExtendedMetaHeader::Patch { application_id, .. } = meta.extended_header {
            Some(application_id)
        } else {
            None
        };
    let mut builders = BTreeMap::new();

    for content in meta.content_info.iter() {
        let content = content.content_info;

        let program_id = ProgramId::new(id_base, content.id_offset);
        let base_program_id =
            base_id_base.map(|base_id_base| ProgramId::new(base_id_base.into(), content.id_offset));
        let builder = builders
            .entry(program_id)
            .or_insert_with(|| ProgramInfoBuilder::new(program_id, base_program_id));

        match content.ty {
            NcmContentType::Program => builder.program_content = Some(content.id),
            NcmContentType::Control => builder.control_content = Some(content.id),
            NcmContentType::HtmlDocument => builder.html_document_content = Some(content.id),

            NcmContentType::Meta
            | NcmContentType::Data
            | NcmContentType::LegalInformation
            | NcmContentType::DeltaFragment => {
                // ignore
            }
        }
    }

    builders
        .into_iter()
        .map(|(program, builder)| {
            builder
                .build(nca_set)
                .context(ProgramsParseSnafu { program })
        })
        .collect()
}
