use crate::fs::nca::structs::Sha256IntegrityInfoLevel;
use crate::fs::storage::{
    BlockAdapterStorage, BlockSliceStorageError, LinearAdapterStorage, ReadableStorage,
    ReadableStorageExt, SharedStorage, SliceStorage, SliceStorageError, StorageError, VecStorage,
};

mod integrity_verification_level_storage;
pub use integrity_verification_level_storage::IntegrityVerificationLevelStorage;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IntegrityStorageType {
    PartitionFs,
    RomFs,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IntegrityCheckLevel {
    /// No integrity checks will be performed.
    None,
    /// Invalid blocks will be marked as invalid when read, and will not cause an error.
    IgnoreOnInvalid,
    /// An error will be returned when an invalid block is read.
    Full,
}

const DIGEST_SIZE: usize = 0x20;

// what an abomination...
// This defined a 2-level hierarchical hash verification thingie.
type PfsVerificationStorage<S> = LinearAdapterStorage<
    IntegrityVerificationLevelStorage<
        BlockAdapterStorage<SliceStorage<SharedStorage<S>>>,
        //
        LinearAdapterStorage<
            IntegrityVerificationLevelStorage<
                BlockAdapterStorage<SliceStorage<SharedStorage<S>>>,
                //
                VecStorage,
            >,
        >,
    >,
>;

#[derive(Debug)]
pub enum NcaVerificationStorage<S: ReadableStorage> {
    Pfs(PfsVerificationStorage<S>),
}

impl<S: ReadableStorage> NcaVerificationStorage<S> {
    pub fn new_pfs_verification_storage(
        storage: S,
        master_hash: [u8; DIGEST_SIZE],
        levels: [Sha256IntegrityInfoLevel; 2],
        block_size: u32,
        integrity_level: IntegrityCheckLevel,
    ) -> Result<Self, SliceStorageError> {
        let master_hash = VecStorage::new(master_hash.to_vec());

        let storage = storage.shared();

        let hash_storage = storage.clone().slice(levels[0].offset, levels[0].size)?;
        let hash_storage = BlockAdapterStorage::new(hash_storage, levels[0].size);

        let data_storage = storage.clone().slice(levels[1].offset, levels[1].size)?;
        let data_storage = BlockAdapterStorage::new(data_storage, block_size as u64);

        Ok(Self::Pfs(LinearAdapterStorage::new(
            IntegrityVerificationLevelStorage::new(
                data_storage,
                LinearAdapterStorage::new(IntegrityVerificationLevelStorage::new(
                    hash_storage,
                    master_hash,
                    integrity_level,
                    IntegrityStorageType::PartitionFs,
                )),
                integrity_level,
                IntegrityStorageType::PartitionFs,
            ),
        )))
    }
}

impl<S: ReadableStorage> ReadableStorage for NcaVerificationStorage<S> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        match self {
            NcaVerificationStorage::Pfs(storage) => storage.read(offset, buf),
        }
    }

    fn get_size(&self) -> u64 {
        match self {
            NcaVerificationStorage::Pfs(storage) => storage.get_size(),
        }
    }
}
