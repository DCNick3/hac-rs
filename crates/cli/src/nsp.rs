use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, Subcommand};
use hac::crypto::keyset::KeySet;
use hac::formats::pfs::PartitionFileSystem;
use hac::snafu::{ErrorCompat, OptionExt, ResultExt, Whatever};
use hac::switch_fs::SwitchFs;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct Opts {
    #[clap(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug)]
enum Action {
    Rename(RenameOpts),
}

#[derive(Args, Debug)]
pub struct RenameOpts {
    directory: PathBuf,
    #[clap(long, default_value = "false")]
    verbose_errors: bool,
    #[clap(long, default_value = "false")]
    ignore_prefix: bool,
}

fn rename_one(opts: &RenameOpts, keys: &KeySet, path: &Utf8Path) -> Result<(), Whatever> {
    static PREFIX_REX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\[[^]]+]").unwrap());

    let pfs = PartitionFileSystem::from_path(path).whatever_context("Opening NSP")?;
    let switch_fs = SwitchFs::new(keys, &pfs).whatever_context("Could not open Switch FS")?;

    let title = switch_fs
        .title_set()
        .values()
        .exactly_one()
        .ok()
        .whatever_context("Could not find exactly one title (some weird NSP?)")?;

    let old_filename = path
        .file_name()
        .whatever_context("Could not get filename")?;

    let prefix = opts
        .ignore_prefix
        .then_some("")
        .or_else(|| PREFIX_REX.find(old_filename).map(|c| c.as_str()))
        .unwrap_or("");

    let new_filename = format!(
        "{}{}{} [{}][v{}].nsp",
        prefix,
        if prefix.is_empty() { "" } else { " " },
        title.any_title().unwrap().name,
        title.title_id(),
        title.version(),
    );

    if new_filename == old_filename {
        return Ok(());
    }

    let new_path = path.with_file_name(&new_filename);
    println!("Renaming {:?} to {:?}", old_filename, new_filename);

    std::fs::rename(path, &new_path)
        .with_whatever_context(|_| format!("Renaming {} to {}", path, new_path))?;

    Ok(())
}

fn rename(opts: RenameOpts) -> Result<(), Whatever> {
    println!("Renaming files in {:?}", &opts.directory);

    let keys = KeySet::from_system(None).unwrap();

    let files = walkdir::WalkDir::new(&opts.directory)
        .into_iter()
        .filter_map(|v| v.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().and_then(OsStr::to_str) == Some("nsp")
        })
        .map(|v| Utf8PathBuf::from_path_buf(v.path().to_owned()))
        .collect::<Result<Vec<_>, _>>()
        .ok()
        .whatever_context(
            "Could not convert some paths to UTF-8, make sure all your filenames are UTF-8",
        )?;

    for file in files {
        if let Err(e) = rename_one(&opts, &keys, &file) {
            if opts.verbose_errors {
                eprintln!("Error renaming {}:", file);
                for e in e.iter_chain() {
                    eprintln!(" - {}", e);
                }
            } else {
                eprintln!("Error renaming {}: {}", file, e);
            }
        }
    }

    Ok(())
}

pub fn main(opts: Opts) -> Result<(), Whatever> {
    match opts.action {
        Action::Rename(opts) => rename(opts),
    }
}
