mod application_set;
mod nca_set;
mod title_set;

use crate::crypto::keyset::KeySet;
use crate::filesystem::ReadableFileSystem;
use snafu::{ResultExt, Snafu};
use std::fmt::Debug;

pub use nca_set::{nca_set_from_fs, NcaSet, NcaSetParseError};
pub use title_set::{
    title_set_from_nca_set, ControlParseError, TitleParseError, TitleSet, TitleSetParseError,
};

#[derive(Snafu, Debug)]
pub enum NewSwitchFsError {
    #[snafu(display("Failed to parse the NCA set"))]
    NcaSetParse { source: NcaSetParseError },
    #[snafu(display("Failed to parse the title set"))]
    TitleSetParse { source: TitleSetParseError },
}

#[derive(Debug)]
pub struct SwitchFs<F: ReadableFileSystem> {
    nca_set: NcaSet<F::Storage>,
    title_set: TitleSet,
}

impl<F: ReadableFileSystem> SwitchFs<F> {
    pub fn new(key_set: &KeySet, fs: &F) -> Result<Self, NewSwitchFsError> {
        let key_set = key_set.clone();

        // TODO: import tickets from the FS

        let nca_set = nca_set_from_fs(&key_set, fs).context(NcaSetParseSnafu)?;
        let title_set = title_set_from_nca_set(&nca_set).context(TitleSetParseSnafu)?;

        Ok(Self { nca_set, title_set })
    }

    pub fn nca_set(&self) -> &NcaSet<F::Storage> {
        &self.nca_set
    }

    pub fn title_set(&self) -> &TitleSet {
        &self.title_set
    }
}
