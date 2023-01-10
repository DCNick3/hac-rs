use hac::crypto::keyset::KeySet;
use hac::fs::nca::{IntegrityCheckLevel, Nca};
use hac::fs::pfs::PartitionFileSystem;
use hac::fs::romfs;
use hac::fs::romfs::RomFileSystem;
use hac::fs::storage::{ReadableStorage, ReadableStorageExt};
use std::path::{Path, PathBuf};

fn walk_romfs<S: ReadableStorage>(dir: romfs::Directory<'_, S>, depth: usize) {
    for entry in dir.entries() {
        match entry {
            romfs::Entry::Directory(dir) => {
                println!("{:indent$}{}", "", dir.name(), indent = depth * 2);
                walk_romfs(dir, depth + 1);
            }
            romfs::Entry::File(file) => {
                println!("{:indent$}{}", "", file.name(), indent = depth * 2);
            }
        }
    }
}

fn extract_romfs<S: ReadableStorage>(dir: romfs::Directory<'_, S>, path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    for entry in dir.entries() {
        match entry {
            romfs::Entry::Directory(dir) => {
                let path = path.join(dir.name());
                std::fs::create_dir_all(&path).unwrap();
                extract_romfs(dir, &path);
            }
            romfs::Entry::File(file) => {
                let path = path.join(file.name());
                let storage = file.storage().unwrap();
                println!("Extracting {}...", path.display());
                storage.save_to_file(path).unwrap();
            }
        }
    }
}

fn main() {
    let base_name = "test_files/de16b5aa443dd171bb90b10b88ec3d3b".to_string();

    let keyset = KeySet::from_system(None).unwrap();
    let nca_storage = hac::fs::storage::FileRoStorage::open(base_name.clone() + ".nca").unwrap();

    let nca = Nca::new(&keyset, nca_storage).unwrap();

    println!("{:#?}", nca);

    let storage = nca
        .get_section_storage(0, IntegrityCheckLevel::Full)
        .unwrap();
    // measure time it took us to write the file
    let start = std::time::Instant::now();
    storage.save_to_file(base_name.clone() + ".0").unwrap();
    let duration = start.elapsed();

    println!("Written the section 0 in {:?}", duration);

    let fs0 = PartitionFileSystem::new(storage).unwrap();

    for file in fs0.iter() {
        let dest = PathBuf::from(base_name.clone() + ".0dir/" + file.filename());
        println!("Extracting {} to {}", file.filename(), dest.display());
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        let storage = file.storage().unwrap();
        storage.save_to_file(dest).unwrap();
    }

    let storage = nca
        .get_section_storage(1, IntegrityCheckLevel::Full)
        .unwrap();
    // measure time it took us to write the file
    let start = std::time::Instant::now();
    storage.save_to_file(base_name.clone() + ".1").unwrap();
    let duration = start.elapsed();

    println!("Written the section 1 in {:?}", duration);

    let fs1 = RomFileSystem::new(storage).unwrap();

    extract_romfs(fs1.root(), &PathBuf::from(base_name.clone() + ".1dir"));

    let storage = nca
        .get_section_storage(2, IntegrityCheckLevel::Full)
        .unwrap();
    // measure time it took us to write the file
    let start = std::time::Instant::now();
    storage.save_to_file(base_name.clone() + ".2").unwrap();
    let duration = start.elapsed();

    println!("Written the section 2 in {:?}", duration);
}
