use hac::crypto::keyset::KeySet;
use hac::filesystem::merge_filesystem::MergeFilesystem;
use hac::filesystem::{
    Entry, ReadableDirectory, ReadableDirectoryExt, ReadableFile, ReadableFileSystem,
};
use hac::formats::nca::{IntegrityCheckLevel, Nca};
use hac::formats::pfs::PartitionFileSystem;
use hac::formats::ticket::Ticket;
use hac::snafu::{ResultExt, Snafu, Whatever};
use hac::storage::ReadableStorageExt;
use hac::switch_fs::content_set::AnyContentInfo;
use hac::switch_fs::SwitchFs;
use itertools::Itertools;
use std::borrow::Cow;
use std::ffi::OsStr;
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

#[derive(Snafu, Debug)]
#[snafu(crate_root(hac::snafu))]
struct Error {
    message: String,
    source: Whatever,
}

#[allow(unused)]
pub fn test_nca() -> Result<(), Whatever> {
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
    Ok(())
}

#[allow(unused)]
pub fn test_ncz() -> Result<(), Whatever> {
    let base_name = "test_files/ncz/block/ed5f53408e88b7d2974e3b6cce8bfa57".to_string();

    let keyset = KeySet::from_system(None).whatever_context("Parsing KeySet")?;
    let ncz_storage = hac::storage::FileRoStorage::open(base_name.clone() + ".ncz")
        .whatever_context("Opening file as storage")?;

    let nca = Nca::new(&keyset, ncz_storage).whatever_context("Parsing NCZ")?;
    // let ncz = Ncz::new(ncz_storage).whatever_context("Parsing NCZ")?;

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
    Ok(())
}

#[allow(unused)]
pub fn test_tik() -> Result<(), Whatever> {
    use hac::binrw::BinRead;

    let file =
        std::fs::read("test_files/fmf_010079300AD54000/010079300ad540000000000000000005.tik")
            .unwrap();
    let mut cursor = std::io::Cursor::new(file);
    let ticket = Ticket::read(&mut cursor).unwrap();

    println!("{:#?}", ticket);
    Ok(())
}

#[allow(unused)]
pub fn test_cnmt() -> Result<(), Whatever> {
    use hac::binrw::BinRead;

    let file = std::fs::read(
        "test_files/e7b074f7535f34c434a1512f776cd0ac.cmnt.0dir/Application_010079300ad54000.cnmt",
    )
    .unwrap();
    let mut cursor = std::io::Cursor::new(file);
    let cnmt = hac::formats::cnmt::PackagedContentMeta::read(&mut cursor).unwrap();

    println!("{:#?}", cnmt);
    Ok(())
}

#[allow(unused)]
pub fn test_nacp() -> Result<(), Whatever> {
    use hac::binrw::BinRead;

    let file = std::fs::read("test_files/0c93fc88e2a0ea63477c6f854a12b457.0dir/control.nacp")
        .whatever_context("Opening nacp")?;
    let mut cursor = std::io::Cursor::new(file);
    let nacp = hac::formats::nacp::ApplicationControlProperty::read(&mut cursor)
        .whatever_context("Reading nacp")?;

    println!("{:#?}", nacp);
    Ok(())
}

#[allow(unused)]
pub fn test_switch_fs() -> Result<(), Whatever> {
    let files = walkdir::WalkDir::new("test_files/nsp")
        .into_iter()
        .filter_map(|v| v.ok())
        .filter(|e| {
            (e.file_type().is_file() || e.file_type().is_symlink())
                && e.path().extension().and_then(OsStr::to_str) == Some("nsp")
        })
        .map(|v| v.path().to_owned())
        .collect::<Vec<_>>();

    println!("Found {} nsp files: {:#?}", files.len(), files);

    // let files = [
    //     "test_files/fmf_010079300AD54000.nsp",
    //     "test_files/fmf_010079300AD54800.nsp",
    // ];

    // let file = "test_files/fmf_010079300AD54000.nsp";
    // let file = "test_files/fmf_010079300AD54800.nsp";
    let keyset = KeySet::from_system(None).whatever_context("Opening system keyset")?;

    let filesystems = files
        .iter()
        .map(|filename| {
            let storage =
                hac::storage::FileRoStorage::open(filename).with_whatever_context(|_| {
                    format!("Opening NSP storage in {}", filename.display())
                })?;
            PartitionFileSystem::new(storage)
                .with_whatever_context(|_| format!("Opening NSP fs in {}", filename.display()))
        })
        .collect::<Result<Vec<_>, Whatever>>()?;

    for (path, fs) in files.iter().zip(filesystems.iter()) {
        for (filename, _) in fs.root().entries_recursive() {
            println!("{:50} -> {}", filename, path.display());
        }
    }

    let merged_fs = MergeFilesystem::new(filesystems);

    println!(
        "Files in the merged FS:\n{:#?}",
        merged_fs
            .root()
            .entries_recursive()
            .flat_map(|(n, e)| e.file().map(|_| n))
            .collect::<Vec<_>>()
    );

    let switch_fs =
        SwitchFs::new(&keyset, &merged_fs).whatever_context("Could not open SwitchFs")?;

    println!("SwitchFs contents:");
    for (&key, content) in switch_fs.title_set().iter().sorted_by_key(|v| v.0) {
        let id = key.id;
        let version = key.version;
        match content {
            AnyContentInfo::Application(app) => {
                let app_title = app.any_title().unwrap();
                println!(
                    "Application {} v{}: {:?} by {:?}",
                    id, version, app_title.name, app_title.publisher,
                );

                for program in app.programs.iter() {
                    let program_title = program.control.any_title().unwrap();
                    println!(
                        "    Program {}: {:?} by {:?}",
                        program.id, program_title.name, program_title.publisher
                    );
                }
            }
            AnyContentInfo::Patch(patch) => {
                let app_title = patch.any_title().unwrap();
                println!(
                    "Patch       {} v{}: {:?} by {:?}",
                    id, version, app_title.name, app_title.publisher,
                );

                for program in patch.programs.iter() {
                    let program_title = program.control.any_title().unwrap();
                    println!(
                        "    Program {}: {:?} by {:?}",
                        program.id, program_title.name, program_title.publisher
                    );
                }
            }
            AnyContentInfo::Data(data) => {
                println!(
                    "Data        {} v{} for {}",
                    id, version, data.application_id
                );
            }
            AnyContentInfo::DataPatch(_) => todo!(),
        }
    }

    println!("SwitchFs applications:");
    for application in switch_fs.application_set().values() {
        let title = application
            .versions
            .values()
            .next()
            .unwrap()
            .programs
            .values()
            .next()
            .unwrap()
            .control
            .any_title()
            .unwrap();
        println!("- [{}] {}", application.id, title.name);
        for version in application.versions.values() {
            for program in version.programs.values() {
                println!(
                    "   {:>16} [{:<8}] [{}]: {}.nca{}",
                    "",
                    format!("{}", version.version),
                    program.id,
                    program.content_id,
                    program
                        .base_content_id
                        .map(|id| format!(" @ {}.nca", id).into())
                        .unwrap_or(Cow::Borrowed(""))
                );
            }
        }

        for addon in application.addons.values() {
            println!(
                "   {:>16} [{:>8}] [{}]: {}.nca",
                "", "AddOn", addon.id, addon.data_content_id,
            );
        }

        println!();
    }

    Ok(())
}

pub fn main() -> Result<(), Whatever> {
    // test_nsp()?;
    // test_nca()?;
    test_ncz()?;
    // test_tik()?;
    // test_cnmt()?;
    // test_nacp()?;
    // test_switch_fs()?;
    Ok(())
}
