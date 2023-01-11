use crate::crypto::keyset::KeySet;
use crate::filesystem::{ReadableDirectoryExt, ReadableFile, ReadableFileSystem};
use crate::formats::ticket::Ticket;
use crate::storage::{ReadableStorageExt, StorageError};
use binrw::BinRead;
use snafu::{ResultExt, Snafu};

#[derive(Snafu, Debug)]
pub enum TicketImportError {
    #[snafu(display("Failed to read the ticket file"))]
    ReadTicketFile { source: StorageError },
    #[snafu(display("Failed to parse the ticket file"))]
    ParseTicketFile { source: binrw::Error },
}

pub fn import_tickets<F: ReadableFileSystem>(
    key_set: &mut KeySet,
    fs: &F,
) -> Result<(), TicketImportError> {
    ReadableDirectoryExt::entries_recursive(&fs.root())
        .filter(|(n, _)| n.ends_with(".tik"))
        .filter_map(|(_, e)| e.file())
        .try_for_each(|file| {
            // it's hard to report this error, as it depends on the FS implementation
            file.storage()
                .expect("Malformed FS")
                .read_all()
                .context(ReadTicketFileSnafu)
                .and_then(|data| {
                    Ticket::read(&mut std::io::Cursor::new(data)).context(ParseTicketFileSnafu)
                })
                .and_then(|ticket| {
                    key_set.import_ticket(&ticket);
                    Ok(())
                })
        })
}
