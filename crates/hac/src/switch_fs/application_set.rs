use crate::ids::{NcaId, TitleId};
use crate::storage::ReadableStorage;
use crate::switch_fs::title_set::Title;
use crate::switch_fs::{NcaSet, TitleSet};
use std::collections::HashMap;

struct ApplicationBuilder<'a>(Vec<&'a Title>);

pub struct Application {
    pub application_title_id: TitleId,
    pub patch_title_id: TitleId,
    pub patch_versions: Vec<u32>,
    pub main_nca_id: NcaId,
    pub patch_nca_ids: Vec<NcaId>,
}

pub type ApplicationSet = HashMap<TitleId, Application>;

pub fn build_application_set<S: ReadableStorage>(
    nca_set: &NcaSet<S>,
    title_set: &TitleSet,
) -> ApplicationSet {
    todo!()
}
