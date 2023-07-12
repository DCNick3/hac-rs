pub mod application_set;
pub mod content_set;
pub mod nca_set;
mod tickets;

use crate::crypto::keyset::KeySet;
use crate::filesystem::ReadableFileSystem;
use snafu::{ResultExt, Snafu};
use std::fmt::Debug;

pub use crate::switch_fs::tickets::{import_tickets, TicketImportError};
use application_set::{build_application_set, ApplicationSet};
use content_set::{
    content_set_from_nca_set, AnyContentInfo, ContentSet, ContentSetParseError, ControlParseError,
    ProgramInfo,
};
use nca_set::{nca_set_from_fs, NcaSet, NcaSetParseError};

#[derive(Snafu, Debug)]
pub enum NewSwitchFsError {
    #[snafu(display("Failed to import ticket"))]
    TicketImport { source: TicketImportError },

    #[snafu(display("Failed to parse the NCA set"))]
    NcaSetParse { source: NcaSetParseError },
    #[snafu(display("Failed to parse the title set"))]
    TitleSetParse { source: ContentSetParseError },
}

#[derive(Debug)]
pub struct SwitchFs<F: ReadableFileSystem> {
    nca_set: NcaSet<F::Storage>,
    title_set: ContentSet,
    application_set: ApplicationSet,
}

impl<F: ReadableFileSystem> SwitchFs<F> {
    pub fn new(key_set: &KeySet, fs: &F) -> Result<Self, NewSwitchFsError> {
        let mut key_set = key_set.clone();

        import_tickets(&mut key_set, fs).context(TicketImportSnafu)?;

        let nca_set = nca_set_from_fs(&key_set, fs).context(NcaSetParseSnafu)?;
        let title_set = content_set_from_nca_set(&nca_set).context(TitleSetParseSnafu)?;
        let application_set = build_application_set(&nca_set, &title_set);

        Ok(Self {
            nca_set,
            title_set,
            application_set,
        })
    }

    pub fn nca_set(&self) -> &NcaSet<F::Storage> {
        &self.nca_set
    }

    pub fn title_set(&self) -> &ContentSet {
        &self.title_set
    }

    pub fn application_set(&self) -> &ApplicationSet {
        &self.application_set
    }
}
