use hac::crypto::keyset::KeySet;
use hac::fs::nca::Nca;

fn main() {
    let keyset = KeySet::from_system(None).unwrap();
    let nca_storage =
        hac::fs::storage::FileRoStorage::open("test_files/de16b5aa443dd171bb90b10b88ec3d3b.nca")
            .unwrap();

    let nca = Nca::new(&keyset, nca_storage).unwrap();

    dbg!(nca);

    todo!()
}
