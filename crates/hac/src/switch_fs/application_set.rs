use crate::formats::cnmt::{ContentMetaType, ExtendedMetaHeader};
use crate::formats::nacp::ApplicationControlProperty;
use crate::ids::{AnyId, ApplicationId, ContentId, DataId, PatchId, ProgramId};
use crate::storage::ReadableStorage;
use crate::switch_fs::{AnyContentInfo, ContentSet, NcaSet, ProgramInfo};
use crate::version::Version;
use indexmap::{IndexMap, IndexSet};
use tracing::warn;

#[derive(Debug)]
pub struct Program {
    pub id: ProgramId,
    pub base_content_id: Option<ContentId>,
    pub content_id: ContentId,

    pub html_document_content_id: Option<ContentId>,
    pub control_content_id: ContentId,
    pub control: ApplicationControlProperty,
}

#[derive(Debug)]
pub enum VersionKind {
    Base,
    Patch,
}

#[derive(Debug)]
pub struct ApplicationVersion {
    pub version: Version,
    pub kind: VersionKind,
    pub meta_content_id: ContentId,
    pub programs: IndexMap<ProgramId, Program>,
}

#[derive(Debug)]
pub struct Addon {
    pub id: DataId,
    pub meta_content_id: ContentId,
    pub data_content_id: ContentId,
}

#[derive(Debug)]
pub struct Application {
    pub id: ApplicationId,
    pub patch_id: PatchId,
    pub base_version: Version,
    pub versions: IndexMap<Version, ApplicationVersion>,
    pub addons: IndexMap<DataId, Addon>,
}

pub type ApplicationSet = IndexMap<ApplicationId, Application>;

pub fn build_application_set<S: ReadableStorage>(
    _nca_set: &NcaSet<S>,
    content_set: &ContentSet,
) -> ApplicationSet {
    let mut applications = IndexMap::new();

    fn make_programs(
        programs: &[ProgramInfo],
        content_id: impl Fn(&ProgramInfo) -> (Option<ContentId>, ContentId),
    ) -> IndexMap<ProgramId, Program> {
        programs
            .iter()
            .map(|pi| {
                let (base_content_id, content_id) = content_id(pi);

                (
                    pi.id,
                    Program {
                        id: pi.id,
                        base_content_id,
                        content_id,
                        html_document_content_id: pi.html_document_content_id,
                        control_content_id: pi.control_content_id,
                        control: pi.control.clone(), // clone is bad =(. Can we share it somehow?
                    },
                )
            })
            .collect()
    }

    for content in content_set.values() {
        match content {
            AnyContentInfo::Application(app) => {
                // create ApplicationVersion for the base version
                let app_version = ApplicationVersion {
                    version: app.common.metadata.version,
                    kind: VersionKind::Base,
                    meta_content_id: app.common.meta_content_id,
                    programs: make_programs(&app.programs, |pi| (None, pi.program_content_id)),
                };

                let application = Application {
                    id: app.id,
                    patch_id: app.patch_id,
                    base_version: app_version.version,
                    versions: IndexMap::from([(app_version.version, app_version)]),
                    addons: Default::default(),
                };

                assert!(applications.insert(app.id, application).is_none());
            }
            _ => {}
        }
    }

    for content in content_set.values() {
        match content {
            AnyContentInfo::Patch(patch) => {
                let Some(app) = applications.get_mut(&patch.application_id) else {
                    warn!(
                        "Patch {:?} references unknown application {:?}",
                        patch.id, patch.application_id
                    );
                    continue;
                };

                let base_version = app
                    .versions
                    .get(&app.base_version)
                    .expect("Could not find base version");

                let app_version = ApplicationVersion {
                    version: patch.common.metadata.version,
                    kind: VersionKind::Patch,
                    meta_content_id: patch.common.meta_content_id,
                    programs: make_programs(&patch.programs, |pi| {
                        (
                            Some(
                                base_version
                                    .programs
                                    .get(&pi.base_program_id.expect("Patch has no base program"))
                                    .expect("Could not find the base program for an update")
                                    .content_id,
                            ),
                            pi.program_content_id,
                        )
                    }),
                };

                assert!(app
                    .versions
                    .insert(app_version.version, app_version)
                    .is_none());
            }
            AnyContentInfo::Data(data) => {
                let Some(app) = applications.get_mut(&data.application_id) else {
                    warn!(
                        "Data {:?} references unknown application {:?}",
                        data.id, data.application_id
                    );
                    continue;
                };

                let addon = Addon {
                    id: data.id,
                    meta_content_id: data.common.meta_content_id,
                    data_content_id: data.data_content,
                };

                assert!(app.addons.insert(data.id, addon).is_none());
            }
            AnyContentInfo::Application(_) => {}
            AnyContentInfo::DataPatch(patch) => {
                warn!("Ignoring data patch {}", patch.id)
            }
        }
    }

    applications
}
