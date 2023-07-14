use crate::crypto::AesKey;
use crate::formats::nca::filesystem::NcaFileSystem;
use crate::formats::nca::structs::{IntegrityInfo, NcaEncryptionType, NcaFormatType};
use crate::formats::nca::{
    IntegrityCheckLevel, Nca, NcaContentKeys, NcaCryptStorage, NcaSectionType,
    NcaVerificationStorage,
};
use crate::storage::{ReadableStorage, ReadableStorageExt, SharedStorage, SliceStorage};

pub type RawEncryptedSectionStorage<S> = SliceStorage<SharedStorage<S>>;
pub type RawDecryptedSectionStorage<S> = NcaCryptStorage<RawEncryptedSectionStorage<S>>;
pub type VerifiedSectionStorage<S> = NcaVerificationStorage<RawDecryptedSectionStorage<S>>;
pub type SectionFileSystem<S> = NcaFileSystem<VerifiedSectionStorage<S>>;

// pub enum RawDecryptedSectionStorage<S: ReadableStorage> {
//     Normal(NcaCryptStorage<RawEncryptedSectionStorage<S>>),
//     Compressed(),
// }

// two layers: (NCA | NCZ) -> (NORMAL | SPARSE(?) | BKTR)

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
        use crate::formats::nca::NcaContentType::Program;
        use crate::formats::nca::NcaSectionType::{Code, Data, Logo};

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
