mod crypt_storage;
pub mod filesystem;
mod structs;
mod verification_storage;

use binrw::BinRead;
use snafu::{ResultExt, Snafu};
use std::io::Cursor;

use crate::crypto::keyset::KeySet;
use crate::crypto::{AesKey, AesXtsKey};
use crate::formats::nca::filesystem::NcaFileSystem;
use crate::formats::nca::structs::{
    IntegrityInfo, NcaEncryptionType, NcaFormatType, NcaFsHeader, NcaHeader, NcaMagic,
};
use crate::storage::{
    ReadableStorage, ReadableStorageExt, SharedStorage, SliceStorage, StorageError,
};

pub use crypt_storage::NcaCryptStorage;
pub use structs::{NcaContentType, NcaSectionType};
pub use verification_storage::{IntegrityCheckLevel, NcaVerificationStorage};

#[derive(Snafu, Debug)]
pub enum NcaError {
    Storage {
        source: StorageError,
    },
    MissingKey {
        source: crate::crypto::keyset::MissingKeyError,
    },
    MissingTitleKey {
        source: crate::crypto::keyset::MissingTitleKeyError,
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
    StorageSizeMismatch {
        expected: u64,
        actual: u64,
    },
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
    storage: SharedStorage<S>,
    headers: AllNcaHeaders,
    content_key: NcaContentKeys,
}

const ALL_HEADERS_SIZE: usize = 0xc00;
const NCA_HEADER_SIZE: usize = 0x400;
const HEADER_SECTOR_SIZE: usize = 0x200;

type RawEncryptedSectionStorage<S> = SliceStorage<SharedStorage<S>>;
type RawDecryptedSectionStorage<S> = NcaCryptStorage<RawEncryptedSectionStorage<S>>;
type VerifiedSectionStorage<S> = NcaVerificationStorage<RawDecryptedSectionStorage<S>>;
type SectionFileSystem<S> = NcaFileSystem<VerifiedSectionStorage<S>>;

impl<S: ReadableStorage> Nca<S> {
    pub fn new(key_set: &KeySet, storage: S) -> Result<Self, NcaError> {
        let (headers, is_decrypted) = Self::parse_headers(key_set, &storage)?;

        if headers.nca_header.nca_size != storage.get_size() {
            return Err(NcaError::StorageSizeMismatch {
                expected: headers.nca_header.nca_size,
                actual: storage.get_size(),
            });
        }

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

        let expected_session_count = if headers.nca_header.content_type == NcaContentType::Program {
            3
        } else {
            1
        };
        assert_eq!(
            headers.fs_headers.iter().flatten().count(),
            expected_session_count
        );

        Ok(Self {
            storage: SharedStorage::new(storage),
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

impl<S: ReadableStorage> Nca<S> {
    pub fn get_raw_encrypted_section_storage(
        &self,
        index: usize,
    ) -> Option<RawEncryptedSectionStorage<S>> {
        let section_entry = self.headers.nca_header.section_table[index];

        if !section_entry.is_enabled {
            return None;
        }

        let fs_header = self.headers.fs_headers[index].as_ref().unwrap();
        if fs_header.exists_sparse_layer() {
            todo!("Sparse layer is not supported yet");
        }

        Some(
            self.storage
                .clone()
                .slice(section_entry.start.into(), section_entry.size())
                .expect("BUG: invalid section slice"),
        )
    }

    fn get_ctr_key(&self) -> AesKey {
        match self.content_key {
            NcaContentKeys::Plaintext => panic!("Attempt to get CTR key for plaintext NCA"),
            NcaContentKeys::KeyArea { ctr: key, .. } | NcaContentKeys::RightsId(key) => key,
        }
    }

    pub fn get_raw_decrypted_section_storage(
        &self,
        index: usize,
    ) -> Option<RawDecryptedSectionStorage<S>> {
        self.get_raw_encrypted_section_storage(index)
            .map(|storage| {
                let fs_header = self.headers.fs_headers[index].as_ref().unwrap();

                if self.is_plaintext() {
                    NcaCryptStorage::Plaintext(storage)
                } else {
                    match fs_header.encryption_type {
                        NcaEncryptionType::Auto => todo!("auto encryption (WTF is this?)"),
                        NcaEncryptionType::None => NcaCryptStorage::Plaintext(storage),
                        NcaEncryptionType::Xts => {
                            todo!("XTS encryption")
                        }
                        NcaEncryptionType::AesCtr => {
                            let key = self.get_ctr_key();
                            let start_offset =
                                self.headers.nca_header.section_table[index].start.into();

                            NcaCryptStorage::new_ctr(
                                storage,
                                key,
                                fs_header.upper_counter,
                                start_offset,
                            )
                        }
                        NcaEncryptionType::AesCtrEx => {
                            todo!("AES-CTR-EX encryption")
                        }
                    }
                }
            })
    }

    pub fn get_section_storage(
        &self,
        index: usize,
        integrity_level: IntegrityCheckLevel,
    ) -> Option<VerifiedSectionStorage<S>> {
        self.get_raw_decrypted_section_storage(index)
            .map(|storage| {
                let fs_header = self.headers.fs_headers[index].as_ref().unwrap();

                if fs_header.exists_compression_layer() {
                    todo!("Compression layer is not supported yet");
                }

                match fs_header.integrity_info {
                    IntegrityInfo::None => todo!("IntegrityInfo::None is not supported yet"),
                    IntegrityInfo::Sha256(s) => {
                        assert_eq!(s.level_count, 2);
                        let levels = s.level_info[..2].try_into().unwrap();

                        NcaVerificationStorage::new_pfs_verification_storage(
                            storage,
                            s.master_hash.0 .0,
                            levels,
                            s.block_size,
                            integrity_level,
                        )
                            .expect("FS header specifies invalid hash level offsets for HierarchicalSha256 integrity verification")
                    }
                    IntegrityInfo::Ivfc(s) => {
                        assert_eq!(s.master_hash_size, 0x20);
                        let master_hash = s.master_hash.0[..0x20].try_into().unwrap();

                        // -1 because the last level is the master hash
                        NcaVerificationStorage::new_ivfc_verification_storage(storage, master_hash, s.level_count - 1, s.level_info, integrity_level)
                            .expect("FS header specifies invalid hash level offsets for IVFC integrity verification")
                    }
                }
            })
    }

    pub fn get_section_fs(
        &self,
        index: usize,
        integrity_level: IntegrityCheckLevel,
    ) -> Option<SectionFileSystem<S>> {
        self.get_section_storage(index, integrity_level)
            .map(|storage| {
                let fs_header = self.headers.fs_headers[index].as_ref().unwrap();

                match fs_header.format_type {
                    NcaFormatType::Romfs => {
                        NcaFileSystem::new_romfs(storage).expect("invalid ROMFS header")
                    }
                    NcaFormatType::Pfs0 => {
                        NcaFileSystem::new_pfs(storage).expect("invalid PFS0 header")
                    }
                }
            })
    }

    pub fn get_section_type(&self, index: usize) -> Option<NcaSectionType> {
        use NcaContentType::Program;
        use NcaSectionType::{Code, Data, Logo};

        match (index, self.headers.nca_header.content_type) {
            (0, Program) => Some(Code),
            (1, Program) => Some(Data),
            (2, Program) => Some(Logo),
            (0, _) => Some(Data),
            _ => None,
        }
    }

    pub fn get_fs(
        &self,
        ty: NcaSectionType,
        integrity_level: IntegrityCheckLevel,
    ) -> Option<SectionFileSystem<S>> {
        let index = (0..4).find(|&i| self.get_section_type(i) == Some(ty))?;

        self.get_section_fs(index, integrity_level)
    }
}
