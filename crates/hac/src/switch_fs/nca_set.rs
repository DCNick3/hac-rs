use crate::crypto::keyset::KeySet;
use crate::filesystem::{ReadableDirectoryExt, ReadableFile, ReadableFileSystem};
use crate::formats::nca::Nca;
use crate::ids::ContentId;
use snafu::{ResultExt, Snafu};
use std::collections::BTreeMap;
use tracing::info;

#[derive(Snafu, Debug)]
pub enum NcaSetParseError {
    NcaParse {
        nca_id: ContentId,
        source: crate::formats::nca::NcaError,
    },
    NcaFilenameParse {
        source: crate::ids::IdParseError,
    },
}

pub type NcaSet<S> = BTreeMap<ContentId, Nca<S>>;

/// Parse an NCA filename
/// Return value of Ok(None) means "doesn't look like an NCA filename"
/// Return value of Err(E) means "looks like an NCA filename, but it's invalid (non-hex chars or wrong length)"
fn parse_nca_filename(filename: &str) -> Result<Option<ContentId>, NcaSetParseError> {
    let filename = filename
        .strip_suffix(".cnmt.nca")
        .or_else(|| filename.strip_suffix(".nca"));

    filename
        .map(|v| v.parse())
        .transpose()
        .context(NcaFilenameParseSnafu)
}

pub fn nca_set_from_fs<F: ReadableFileSystem>(
    key_set: &KeySet,
    fs: &F,
) -> Result<NcaSet<F::Storage>, NcaSetParseError> {
    let mut ncas = BTreeMap::new();

    for file in ReadableDirectoryExt::entries_recursive(&fs.root())
        .filter(|(n, _)| n.ends_with(".nca"))
        .filter_map(|(_, e)| e.file())
    {
        // it's hard to report this error, as it depends on the FS implementation
        // TODO: figure it out, without a panic
        let storage = file.storage().expect("Malformed FS");
        let nca_id = parse_nca_filename(file.name())?.expect("BUG: non-NCA filename not filtered");
        info!("Parsing NCA {}", nca_id);
        let nca = Nca::new(key_set, storage).context(NcaParseSnafu { nca_id })?;
        ncas.insert(nca_id, nca);
    }

    Ok(ncas)
}
