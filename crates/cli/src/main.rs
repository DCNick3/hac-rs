use hac::crypto::keyset::KeySet;
use hac::filesystem::{
    Entry, ReadableDirectory, ReadableDirectoryExt, ReadableFile, ReadableFileSystem,
};
use hac::formats::nca::{IntegrityCheckLevel, Nca};
use hac::formats::pfs::PartitionFileSystem;
use hac::storage::ReadableStorageExt;
use hac::switch_fs::SwitchFs;
use hac::ticket::Ticket;
use std::path::{Path, PathBuf};

#[allow(unused)]
fn walk_fs(root_dir: impl ReadableDirectory, depth: usize) {
    for entry in root_dir.entries() {
        match entry {
            Entry::Directory(dir) => {
                println!("{:indent$}{}", "", dir.name(), indent = depth * 2);
                walk_fs(dir, depth + 1);
            }
            Entry::File(file) => {
                println!("{:indent$}{}", "", file.name(), indent = depth * 2);
            }
        }
    }
}

fn extract_fs(root_dir: impl ReadableDirectory, path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    for entry in root_dir.entries() {
        match entry {
            Entry::Directory(dir) => {
                let path = path.join(dir.name());
                std::fs::create_dir_all(&path).unwrap();
                extract_fs(dir, &path);
            }
            Entry::File(file) => {
                let path = path.join(file.name());
                let storage = file.storage().unwrap();
                // println!("Extracting {}...", path.display());
                storage.save_to_file(path).unwrap();
            }
        }
    }
}

#[allow(unused)]
fn test_nca() {
    let base_name = "test_files/de16b5aa443dd171bb90b10b88ec3d3b".to_string();

    let keyset = KeySet::from_system(None).unwrap();
    let nca_storage = hac::storage::FileRoStorage::open(base_name.clone() + ".nca").unwrap();

    let nca = Nca::new(&keyset, nca_storage).unwrap();

    println!("{:#?}", nca);

    let start = std::time::Instant::now();
    let fs0 = nca.get_section_fs(0, IntegrityCheckLevel::Full).unwrap();
    extract_fs(fs0.root(), &PathBuf::from(base_name.clone() + ".0dir"));
    let duration = start.elapsed();

    println!("Written the section 0 in {:?}", duration);

    // measure time it took us to write the file
    let start = std::time::Instant::now();
    let fs1 = nca.get_section_fs(1, IntegrityCheckLevel::Full).unwrap();
    extract_fs(fs1.root(), &PathBuf::from(base_name.clone() + ".1dir"));
    let duration = start.elapsed();

    println!("Written the section 1 in {:?}", duration);

    // measure time it took us to write the file
    let start = std::time::Instant::now();
    let fs2 = nca.get_section_fs(2, IntegrityCheckLevel::Full).unwrap();
    extract_fs(fs2.root(), &PathBuf::from(base_name.clone() + ".2dir"));
    let duration = start.elapsed();

    println!("Written the section 2 in {:?}", duration);
}

#[allow(unused)]
fn test_tik() {
    use hac::binrw::BinRead;

    let file =
        std::fs::read("test_files/fmf_010079300AD54000/010079300ad540000000000000000005.tik")
            .unwrap();
    let mut cursor = std::io::Cursor::new(file);
    let ticket = Ticket::read(&mut cursor).unwrap();

    println!("{:#?}", ticket);
}

#[allow(unused)]
fn test_cnmt() {
    use hac::binrw::BinRead;

    let file = std::fs::read(
        "test_files/e7b074f7535f34c434a1512f776cd0ac.cmnt.0dir/Application_010079300ad54000.cnmt",
    )
    .unwrap();
    let mut cursor = std::io::Cursor::new(file);
    let cnmt = hac::formats::cnmt::Cnmt::read(&mut cursor).unwrap();

    println!("{:#?}", cnmt);
}

#[allow(unused)]
fn test_nacp() {
    use hac::binrw::BinRead;

    let file =
        std::fs::read("test_files/0c93fc88e2a0ea63477c6f854a12b457.0dir/control.nacp").unwrap();
    let mut cursor = std::io::Cursor::new(file);
    let nacp = hac::formats::nacp::Nacp::read(&mut cursor).unwrap();

    println!("{:#?}", nacp);
}

fn main() {
    tracing_subscriber::fmt::init();

    let file = "test_files/fmf_010079300AD54000.nsp";
    let keyset = KeySet::from_system(None).unwrap();

    let nsp_storage = hac::storage::FileRoStorage::open(file).unwrap();

    let nsp = PartitionFileSystem::new(nsp_storage).unwrap();

    println!(
        "Files in the NSP:\n{:#?}",
        nsp.root()
            .entries_recursive()
            .flat_map(|(n, e)| e.file().map(|_| n))
            .collect::<Vec<_>>()
    );

    let switch_fs = SwitchFs::new(&keyset, &nsp).unwrap();

    println!("{:#?}", switch_fs);
}
