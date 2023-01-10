use hac::crypto::keyset::KeySet;
use hac::fs::nca::Nca;
use hac::fs::storage::ReadableStorageExt;

fn main() {
    let keyset = KeySet::from_system(None).unwrap();
    let nca_storage =
        hac::fs::storage::FileRoStorage::open("test_files/de16b5aa443dd171bb90b10b88ec3d3b.nca")
            .unwrap();

    let nca = Nca::new(&keyset, nca_storage).unwrap();

    println!("{:#?}", nca);

    let storage = nca.get_decrypted_section_storage(0).unwrap();

    storage
        .save_to_file("test_files/de16b5aa443dd171bb90b10b88ec3d3b.0")
        .unwrap();
}
