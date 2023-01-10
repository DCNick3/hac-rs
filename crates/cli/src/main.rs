use hac::crypto::keyset::KeySet;
use hac::fs::nca::{IntegrityCheckLevel, Nca};
use hac::fs::pfs::PartitionFileSystem;
use hac::fs::storage::ReadableStorageExt;
use std::path::PathBuf;

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

    let storage = nca
        .get_section_storage(2, IntegrityCheckLevel::Full)
        .unwrap();
    // measure time it took us to write the file
    let start = std::time::Instant::now();
    storage.save_to_file(base_name.clone() + ".2").unwrap();
    let duration = start.elapsed();

    println!("Written the section 2 in {:?}", duration);
}
