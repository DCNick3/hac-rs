use crate::formats::nca::structs::{IvfcIntegrityInfoLevel, Sha256IntegrityInfoLevel};
use crate::storage::{
    BlockAdapterStorage, LinearAdapterStorage, ReadableStorage, ReadableStorageExt, SharedStorage,
    SliceStorage, SliceStorageError, StorageError, VecStorage,
};

mod integrity_verification_level_storage;
pub use integrity_verification_level_storage::IntegrityVerificationLevelStorage;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IntegrityStorageType {
    HierarchicalSha256,
    Ivfc,
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

type AddLevel<S, B> = LinearAdapterStorage<
    IntegrityVerificationLevelStorage<BlockAdapterStorage<SliceStorage<SharedStorage<S>>>, B>,
>;

type VerificationStorage1<S> = AddLevel<S, VecStorage>;
type VerificationStorage2<S> = AddLevel<S, VerificationStorage1<S>>;
type VerificationStorage3<S> = AddLevel<S, VerificationStorage2<S>>;
type VerificationStorage4<S> = AddLevel<S, VerificationStorage3<S>>;
type VerificationStorage5<S> = AddLevel<S, VerificationStorage4<S>>;
type VerificationStorage6<S> = AddLevel<S, VerificationStorage5<S>>;

#[derive(Debug)]
pub enum NcaVerificationStorage<S: ReadableStorage> {
    Level1(VerificationStorage1<S>),
    Level2(VerificationStorage2<S>),
    Level3(VerificationStorage3<S>),
    Level4(VerificationStorage4<S>),
    Level5(VerificationStorage5<S>),
    Level6(VerificationStorage6<S>),
}

#[derive(Debug, Copy, Clone)]
struct LevelInfo {
    offset: u64,
    size: u64,
    block_size: u32,
}

impl From<IvfcIntegrityInfoLevel> for LevelInfo {
    fn from(v: IvfcIntegrityInfoLevel) -> Self {
        Self {
            offset: v.offset,
            size: v.size,
            block_size: 1 << v.block_size,
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct VerificationParams {
    integrity_level: IntegrityCheckLevel,
    ty: IntegrityStorageType,
}

fn add_level<S: ReadableStorage, B: ReadableStorage>(
    base_storage: SharedStorage<S>,
    hash_storage: B,
    level: LevelInfo,
    params: VerificationParams,
) -> Result<AddLevel<S, B>, SliceStorageError> {
    let data_storage = BlockAdapterStorage::new(
        SliceStorage::new(base_storage, level.offset, level.size)?,
        level.block_size as u64,
    );

    Ok(LinearAdapterStorage::new(
        IntegrityVerificationLevelStorage::new(
            data_storage,
            hash_storage,
            params.integrity_level,
            params.ty,
        ),
    ))
}

fn make_level1_storage<S: ReadableStorage>(
    storage: SharedStorage<S>,
    master_hash: [u8; DIGEST_SIZE],
    levels: [LevelInfo; 1],
    params: VerificationParams,
) -> Result<VerificationStorage1<S>, SliceStorageError> {
    let [_levels @ .., level] = levels;
    let hash_storage = VecStorage::new(master_hash.into());
    add_level(storage, hash_storage, level, params)
}

macro_rules! make_level_storage {
    ($name:ident, $level:literal, $res:ident, $prev:ident) => {
        fn $name<S: ReadableStorage>(
            storage: SharedStorage<S>,
            master_hash: [u8; DIGEST_SIZE],
            levels: [LevelInfo; $level],
            params: VerificationParams,
        ) -> Result<$res<S>, SliceStorageError> {
            let [levels @ .., level] = levels;
            let hash_storage = $prev(storage.clone(), master_hash, levels, params)?;
            add_level(storage, hash_storage, level, params)
        }
    };
}

make_level_storage!(
    make_level2_storage,
    2,
    VerificationStorage2,
    make_level1_storage
);
make_level_storage!(
    make_level3_storage,
    3,
    VerificationStorage3,
    make_level2_storage
);
make_level_storage!(
    make_level4_storage,
    4,
    VerificationStorage4,
    make_level3_storage
);
make_level_storage!(
    make_level5_storage,
    5,
    VerificationStorage5,
    make_level4_storage
);
make_level_storage!(
    make_level6_storage,
    6,
    VerificationStorage6,
    make_level5_storage
);

impl<S: ReadableStorage> NcaVerificationStorage<S> {
    pub fn new_pfs_verification_storage(
        storage: S,
        master_hash: [u8; DIGEST_SIZE],
        levels: [Sha256IntegrityInfoLevel; 2],
        block_size: u32,
        integrity_level: IntegrityCheckLevel,
    ) -> Result<Self, SliceStorageError> {
        let params = VerificationParams {
            integrity_level,
            ty: IntegrityStorageType::HierarchicalSha256,
        };

        Ok(Self::Level2(make_level2_storage(
            storage.shared(),
            master_hash,
            [
                LevelInfo {
                    offset: levels[0].offset,
                    size: levels[0].size,
                    block_size: levels[0].size as u32,
                },
                LevelInfo {
                    offset: levels[1].offset,
                    size: levels[1].size,
                    block_size,
                },
            ],
            params,
        )?))
    }

    pub fn new_ivfc_verification_storage(
        storage: S,
        master_hash: [u8; DIGEST_SIZE],
        level_count: u32,
        levels: [IvfcIntegrityInfoLevel; 6],
        integrity_level: IntegrityCheckLevel,
    ) -> Result<Self, SliceStorageError> {
        let params = VerificationParams {
            integrity_level,
            ty: IntegrityStorageType::Ivfc,
        };

        let levels: [LevelInfo; 6] = levels
            .iter()
            .map(|level| (*level).into())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        Ok(match level_count {
            1 => Self::Level1(make_level1_storage(
                storage.shared(),
                master_hash,
                [levels[0]],
                params,
            )?),
            2 => Self::Level2(make_level2_storage(
                storage.shared(),
                master_hash,
                [levels[0], levels[1]],
                params,
            )?),
            3 => Self::Level3(make_level3_storage(
                storage.shared(),
                master_hash,
                [levels[0], levels[1], levels[2]],
                params,
            )?),
            4 => Self::Level4(make_level4_storage(
                storage.shared(),
                master_hash,
                [levels[0], levels[1], levels[2], levels[3]],
                params,
            )?),
            5 => Self::Level5(make_level5_storage(
                storage.shared(),
                master_hash,
                [levels[0], levels[1], levels[2], levels[3], levels[4]],
                params,
            )?),
            6 => Self::Level6(make_level6_storage(
                storage.shared(),
                master_hash,
                [
                    levels[0], levels[1], levels[2], levels[3], levels[4], levels[5],
                ],
                params,
            )?),
            l => panic!("Invalid level count {}", l),
        })
    }
}

impl<S: ReadableStorage> ReadableStorage for NcaVerificationStorage<S> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        match self {
            Self::Level1(storage) => storage.read(offset, buf),
            Self::Level2(storage) => storage.read(offset, buf),
            Self::Level3(storage) => storage.read(offset, buf),
            Self::Level4(storage) => storage.read(offset, buf),
            Self::Level5(storage) => storage.read(offset, buf),
            Self::Level6(storage) => storage.read(offset, buf),
        }
    }

    fn get_size(&self) -> u64 {
        match self {
            Self::Level1(storage) => storage.get_size(),
            Self::Level2(storage) => storage.get_size(),
            Self::Level3(storage) => storage.get_size(),
            Self::Level4(storage) => storage.get_size(),
            Self::Level5(storage) => storage.get_size(),
            Self::Level6(storage) => storage.get_size(),
        }
    }
}
