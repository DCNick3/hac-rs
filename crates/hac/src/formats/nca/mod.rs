mod contents;
mod crypt_storage;
pub mod filesystem;
mod ncz;
mod structs;
mod verification_storage;

use binrw::BinRead;
use itertools::Either;
use snafu::{ResultExt, Snafu};
use std::io::Cursor;

use crate::crypto::keyset::KeySet;
use crate::crypto::{AesKey, AesXtsKey};
use crate::formats::nca::structs::{NcaFsHeader, NcaHeader, NcaMagic};
use crate::storage::{ReadableStorage, ReadableStorageExt, StorageError};

pub use contents::{
    RawDecryptedSectionStorage, RawEncryptedSectionStorage, SectionFileSystem,
    VerifiedSectionStorage,
};
pub use crypt_storage::NcaCryptStorage;
pub use structs::{NcaContentType, NcaSectionType};
pub use verification_storage::{IntegrityCheckLevel, NcaVerificationStorage};

use crate::formats::nca::contents::Body;
use crate::formats::nca::ncz::NczBodyStorage;
pub use ncz::NczError;

#[derive(Snafu, Debug)]
pub enum NcaError {
    /// NCA: Failed to read from the storage
    Storage { source: StorageError },
    /// NCA: Missing a crypto key
    MissingKey {
        source: crate::crypto::keyset::MissingKeyError,
    },
    /// NCA: Missing a title key
    MissingTitleKey {
        source: crate::crypto::keyset::MissingTitleKeyError,
    },
    /// NCA: Failed to parse the NCA header
    NcaHeaderParsing { source: binrw::Error },
    /// NCA: Failed to parse the NCA FS header for section {index}
    FsHeaderParsing { index: usize, source: binrw::Error },
    /// NCA: Error while handling an NCZ file
    Ncz { source: NczError },
    /// NCA: FS header hash mismatch for section {index}
    FsHeaderHashMismatch { index: usize },
    /// NCA: Invalid size: expected {expected}, got {actual}
    StorageSizeMismatch { expected: u64, actual: u64 },
}

#[derive(Debug)]
struct AllNcaHeaders {
    pub nca_header: NcaHeader,
    pub fs_headers: [Option<NcaFsHeader>; 4],
}

impl AllNcaHeaders {
    pub fn has_rights_id(&self) -> bool {
        !self.nca_header.rights_id.is_empty()
    }

    pub fn master_key_revision(&self) -> u8 {
        std::cmp::max(
            self.nca_header.key_generation_1,
            self.nca_header.key_generation_2,
        )
        .saturating_sub(1)
    }
}

#[derive(Debug)]
enum NcaContentKeys {
    /// NCA is decrypted, no keys are needed.
    Plaintext,
    /// Keys that were decrypted from the key area for Normal crypto
    #[allow(dead_code)] // TODO: implement key area decryption, then this will be used
    KeyArea { ctr: AesKey, xts: AesXtsKey },
    /// Decrypted key for the RightsId crypto obtained externally
    RightsId(AesKey),
}

#[derive(Debug)]
pub struct Nca<S: ReadableStorage> {
    body: Body<S>,
    headers: AllNcaHeaders,
    content_key: NcaContentKeys,
}

const ALL_HEADERS_SIZE: usize = 0xc00;
const NCA_HEADER_SIZE: usize = 0x400;
const HEADER_SECTOR_SIZE: usize = 0x200;

impl<S: ReadableStorage> Nca<S> {
    pub fn new(key_set: &KeySet, storage: S) -> Result<Self, NcaError> {
        let (headers, is_decrypted) = Self::parse_headers(key_set, &storage)?;

        let content_key = if is_decrypted {
            NcaContentKeys::Plaintext
        } else if headers.has_rights_id() {
            let title_key = key_set
                .title_key(&headers.nca_header.rights_id)
                .context(MissingTitleKeySnafu)?;

            let title_kek = key_set
                .title_kek(headers.master_key_revision())
                .context(MissingKeySnafu)?;

            NcaContentKeys::RightsId(title_key.decrypt(title_kek))
        } else {
            let kak = key_set
                .key_area_key(
                    headers.master_key_revision(),
                    headers.nca_header.key_area_key_index,
                )
                .context(MissingKeySnafu)?;

            let ctr = kak.decrypt_key(headers.nca_header.key_area.encrypted_ctr_key);
            let xts = kak.decrypt_xts_key(headers.nca_header.key_area.encrypted_xts_key);

            NcaContentKeys::KeyArea { ctr, xts }
        };

        let section_count = headers.fs_headers.iter().flatten().count();
        if headers.nca_header.content_type == NcaContentType::Program {
            assert!(matches!(section_count, 2 | 3)); // base NCA contain 3 sections, update NCA contain 2 sections (w/o the logo)
        } else {
            assert_eq!(section_count, 1);
        };

        let body = match NczBodyStorage::try_new(storage).context(NczSnafu)? {
            Either::Left(ncz_storage) => Body::Ncz(ncz_storage.shared()),
            Either::Right(storage) => Body::Nca(storage.shared()),
        };

        if headers.nca_header.nca_size != body.get_size() {
            return Err(NcaError::StorageSizeMismatch {
                expected: headers.nca_header.nca_size,
                actual: body.get_size(),
            });
        }

        Ok(Self {
            body,
            headers,
            content_key,
        })
    }

    pub fn is_plaintext(&self) -> bool {
        matches!(self.content_key, NcaContentKeys::Plaintext)
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

            if section_entry.is_enabled {
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
impl<S: ReadableStorage> Nca<S> {
    pub fn content_type(&self) -> NcaContentType {
        self.headers.nca_header.content_type
    }
}
