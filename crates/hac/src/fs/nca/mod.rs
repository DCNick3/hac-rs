mod structs;

use crate::crypto::keyset::KeySet;
use crate::fs::nca::structs::{NcaFsHeader, NcaHeader, NcaMagic};
use crate::fs::storage::{Storage, StorageError};
use binrw::BinRead;
use snafu::{ResultExt, Snafu};
use std::io::Cursor;

#[derive(Snafu, Debug)]
pub enum NcaError {
    Storage {
        source: StorageError,
    },
    MissingKey {
        source: crate::crypto::keyset::MissingKeyError,
    },
    NcaHeaderParsing {
        source: binrw::Error,
    },
    FsHeaderParsing {
        index: usize,
        source: binrw::Error,
    },
    FsHeaderHashMismatch {
        index: usize,
    },
}

#[derive(Debug)]
struct AllNcaHeaders {
    pub nca_header: NcaHeader,
    pub fs_headers: [Option<NcaFsHeader>; 4],
}

#[derive(Debug)]
pub struct Nca<S: Storage> {
    storage: S,
    headers: AllNcaHeaders,
    is_decrypted: bool,
}

const ALL_HEADERS_SIZE: usize = 0xc00;
const NCA_HEADER_SIZE: usize = 0x400;
const HEADER_SECTOR_SIZE: usize = 0x200;

impl<S: Storage> Nca<S> {
    pub fn new(key_set: &KeySet, storage: S) -> Result<Self, NcaError> {
        let (headers, is_decrypted) = Self::parse_headers(key_set, &storage)?;

        Ok(Self {
            storage,
            headers,
            is_decrypted,
        })
    }

    fn try_parse_nca_header(header: &[u8]) -> Result<NcaHeader, NcaError> {
        assert_eq!(header.len(), NCA_HEADER_SIZE);
        let mut cur = Cursor::new(header);

        let res = NcaHeader::read(&mut cur).context(NcaHeaderParsingSnafu)?;
        assert_eq!(cur.position(), NCA_HEADER_SIZE as u64);
        Ok(res)
    }

    /// Just do the decryption, don't parse the full header yet.
    fn parse_headers(key_set: &KeySet, storage: &S) -> Result<(AllNcaHeaders, bool), NcaError> {
        let mut headers_data = [0; ALL_HEADERS_SIZE];
        storage.read(0, &mut headers_data).context(StorageSnafu)?;

        let (nca_header_data, fs_header_data) = headers_data.split_at_mut(NCA_HEADER_SIZE);

        let mut is_decrypted = false;

        let nca_header = if let Ok(nca_header) = Self::try_parse_nca_header(nca_header_data) {
            // if we were able to parse the NCA header, chances are it's not encrypted
            is_decrypted = true;
            nca_header
        } else {
            // else - perform the decryption
            let key = key_set.header_key().context(MissingKeySnafu)?;

            key.decrypt(nca_header_data, 0, HEADER_SECTOR_SIZE);

            let nca_header = Self::try_parse_nca_header(nca_header_data)?;

            match nca_header.magic {
                NcaMagic::Nca0 => unimplemented!("NCA0 decryption"),
                NcaMagic::Nca1 => unimplemented!("NCA1 decryption"),
                NcaMagic::Nca2 => {
                    for i in 0..4 {
                        // Nca2 encrypts fs headers each as it was a sector 0 (for some godforsaken reason)
                        key.decrypt(
                            &mut fs_header_data[i * HEADER_SECTOR_SIZE..],
                            0,
                            HEADER_SECTOR_SIZE,
                        );
                    }
                }
                NcaMagic::Nca3 => {
                    // decrypt the rest with normal sector numbers
                    key.decrypt(fs_header_data, 2, HEADER_SECTOR_SIZE);
                }
            }

            nca_header
        };

        // TODO: here we ignore the header signature, probably we should check it

        let mut fs_headers = [None; 4];
        // parse the section fs headers
        for (index, data) in fs_header_data.chunks_exact(HEADER_SECTOR_SIZE).enumerate() {
            let section_entry = nca_header.section_table[index];

            if section_entry.present() {
                let hash = nca_header.fs_header_hashes[index];
                hash.verify(data)
                    .map_err(|_| NcaError::FsHeaderHashMismatch { index })?;

                let mut cur = Cursor::new(data);

                fs_headers[index] =
                    Some(NcaFsHeader::read(&mut cur).context(FsHeaderParsingSnafu { index })?);
                assert_eq!(cur.position(), HEADER_SECTOR_SIZE as u64);
            }
        }

        Ok((
            AllNcaHeaders {
                nca_header,
                fs_headers,
            },
            is_decrypted,
        ))
    }
}
