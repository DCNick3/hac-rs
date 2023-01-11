mod application_set;
mod nca_set;
mod tickets;
mod title_set;

use crate::crypto::keyset::KeySet;
use crate::filesystem::ReadableFileSystem;
use snafu::{ResultExt, Snafu};
use std::fmt::Debug;

pub use crate::switch_fs::tickets::{import_tickets, TicketImportError};
pub use application_set::{build_application_set, Application, ApplicationSet};
pub use nca_set::{nca_set_from_fs, NcaSet, NcaSetParseError};
pub use title_set::{
    title_set_from_nca_set, ControlParseError, TitleParseError, TitleSet, TitleSetParseError,
};

#[derive(Snafu, Debug)]
pub enum NewSwitchFsError {
    #[snafu(display("Failed to import ticket"))]
    TicketImport { source: TicketImportError },

    #[snafu(display("Failed to parse the NCA set"))]
    NcaSetParse { source: NcaSetParseError },
    #[snafu(display("Failed to parse the title set"))]
    TitleSetParse { source: TitleSetParseError },
}

#[derive(Debug)]
pub struct SwitchFs<F: ReadableFileSystem> {
    nca_set: NcaSet<F::Storage>,
    title_set: TitleSet,
    application_set: ApplicationSet,
}

impl<F: ReadableFileSystem> SwitchFs<F> {
    pub fn new(key_set: &KeySet, fs: &F) -> Result<Self, NewSwitchFsError> {
        let mut key_set = key_set.clone();

        import_tickets(&mut key_set, fs).context(TicketImportSnafu)?;

        let nca_set = nca_set_from_fs(&key_set, fs).context(NcaSetParseSnafu)?;
        let title_set = title_set_from_nca_set(&nca_set).context(TitleSetParseSnafu)?;
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

    pub fn title_set(&self) -> &TitleSet {
        &self.title_set
    }

    pub fn application_set(&self) -> &ApplicationSet {
        &self.application_set
    }
}
