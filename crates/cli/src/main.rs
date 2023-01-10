use hac::crypto::keyset::KeySet;
use hac::fs::nca::{IntegrityCheckLevel, Nca};
use hac::fs::storage::ReadableStorageExt;

fn main() {
    let keyset = KeySet::from_system(None).unwrap();
    let nca_storage =
        hac::fs::storage::FileRoStorage::open("test_files/de16b5aa443dd171bb90b10b88ec3d3b.nca")
            .unwrap();

    let nca = Nca::new(&keyset, nca_storage).unwrap();

    println!("{:#?}", nca);

    let storage = nca
        .get_section_storage(0, IntegrityCheckLevel::Full)
        .unwrap();
    // measure time it took us to write the file
    let start = std::time::Instant::now();
    storage
        .save_to_file("test_files/de16b5aa443dd171bb90b10b88ec3d3b.0")
        .unwrap();
    let duration = start.elapsed();

    println!("Written the section 0 in {:?}", duration);

    let storage = nca
        .get_section_storage(1, IntegrityCheckLevel::Full)
        .unwrap();
    // measure time it took us to write the file
    let start = std::time::Instant::now();
    storage
        .save_to_file("test_files/de16b5aa443dd171bb90b10b88ec3d3b.1")
        .unwrap();
    let duration = start.elapsed();

    println!("Written the section 1 in {:?}", duration);

    let storage = nca
        .get_section_storage(2, IntegrityCheckLevel::Full)
        .unwrap();
    // measure time it took us to write the file
    let start = std::time::Instant::now();
    storage
        .save_to_file("test_files/de16b5aa443dd171bb90b10b88ec3d3b.2")
        .unwrap();
    let duration = start.elapsed();

    println!("Written the section 2 in {:?}", duration);
}
