use crate::formats::cnmt::{ContentMetaType, TypeSpecificContentMeta};
use crate::ids::{NcaId, TitleId};
use crate::storage::ReadableStorage;
use crate::switch_fs::{NcaSet, TitleSet};
use indexmap::{IndexMap, IndexSet};

#[derive(Debug)]
pub struct ApplicationPatch {
    // TODO: this is kinda simplistic
    // the CNMT contain the precise definition: which NCA patches/replaces which other NCA etc
    pub main_nca_id: NcaId,
    pub version: u32,
}

#[derive(Debug)]
pub struct Application {
    pub application_title_id: TitleId,
    pub patch_title_id: TitleId,
    pub main_nca_id: NcaId,
    pub patches: Vec<ApplicationPatch>,
}

pub type ApplicationSet = IndexMap<TitleId, Application>;

pub fn build_application_set<S: ReadableStorage>(
    _nca_set: &NcaSet<S>,
    title_set: &TitleSet,
) -> ApplicationSet {
    let mut interested_patch_ids = IndexSet::new();
    let mut applications = IndexMap::new();

    // find the applications
    for title in title_set.values() {
        if title.ty() == ContentMetaType::Application {
            let patch_title_id =
                if let TypeSpecificContentMeta::Application { patch_title_id, .. } =
                    title.metadata.type_specific
                {
                    patch_title_id
                } else {
                    unreachable!()
                };

            let application = Application {
                application_title_id: title.title_id(),
                patch_title_id,
                main_nca_id: title.main_nca_id.expect("FIXME"), // TODO
                patches: vec![],
            };

            applications.insert(title.title_id(), application);
            interested_patch_ids.insert(patch_title_id);
        }
    }

    // find the patches
    for (_, title) in title_set {
        if interested_patch_ids.contains(&title.title_id()) {
            let application_title_id = if let TypeSpecificContentMeta::Patch {
                application_title_id,
                ..
            } = title.metadata.type_specific
            {
                application_title_id
            } else {
                panic!("Patch TitleId specified a non-patch title")
            };

            let application = applications.get_mut(&application_title_id).unwrap();
            application.patches.push(ApplicationPatch {
                main_nca_id: title.main_nca_id.expect("FIXME"), // TODO
                version: title.version(),
            });
        }
    }

    for application in applications.values_mut() {
        application.patches.sort_by_key(|v| v.version)
    }

    applications
}
