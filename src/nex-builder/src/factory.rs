//! Factory defaults for Nex systems whose writable `/etc` lives outside the image.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Clone, Eq, PartialEq)]
enum Entry {
    Directory(u32),
    File(u32, blake3::Hash),
    Symlink(PathBuf),
    Other(u32),
}

#[derive(Default)]
pub(crate) struct Snapshot(BTreeMap<PathBuf, Entry>);

pub(crate) fn capture(target: &Path) -> io::Result<Snapshot> {
    let root = target.join("usr/share/factory/etc");
    let mut entries = BTreeMap::new();
    if root.is_dir() {
        capture_children(&root, &root, &mut entries)?;
    }
    Ok(Snapshot(entries))
}

pub(crate) fn move_etc(target: &Path, base: Option<&Snapshot>) -> io::Result<()> {
    let source = target.join("etc");
    let factory = target.join("usr/share/factory/etc");
    if !source.exists() {
        fs::create_dir_all(source)?;
        return Ok(());
    }
    if factory.exists() {
        merge(&source, &factory, &factory, base)?;
        fs::remove_dir(&source)?;
    } else {
        fs::create_dir_all(factory.parent().expect("factory path has parent"))?;
        fs::rename(&source, &factory)?;
    }
    fs::create_dir(&source)
}

fn merge(
    source: &Path,
    destination: &Path,
    root: &Path,
    base: Option<&Snapshot>,
) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let source_type = entry.file_type()?;
        let target_type = match to.symlink_metadata() {
            Ok(metadata) => metadata.file_type(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::rename(from, to)?;
                continue;
            }
            Err(error) => return Err(error),
        };
        if source_type.is_dir() && target_type.is_dir() {
            merge(&from, &to, root, base)?;
            fs::remove_dir(from)?;
            continue;
        }
        let replaces_base = base
            .map(|snapshot| unchanged(&to, root, snapshot))
            .transpose()?
            .unwrap_or(false);
        if replaces_base {
            remove(&to, target_type)?;
            fs::rename(from, to)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "legacy /etc default conflicts with factory file {}",
                    to.display()
                ),
            ));
        }
    }
    Ok(())
}

fn unchanged(path: &Path, root: &Path, base: &Snapshot) -> io::Result<bool> {
    let relative = path
        .strip_prefix(root)
        .expect("factory path stays under root");
    let mut actual = BTreeMap::new();
    capture_path(root, path, &mut actual)?;
    let expected = base
        .0
        .iter()
        .filter(|(candidate, _)| candidate.as_path() == relative || candidate.starts_with(relative))
        .map(|(path, entry)| (path.clone(), entry.clone()))
        .collect::<BTreeMap<_, _>>();
    Ok(!expected.is_empty() && actual == expected)
}

fn capture_children(
    root: &Path,
    directory: &Path,
    entries: &mut BTreeMap<PathBuf, Entry>,
) -> io::Result<()> {
    for child in fs::read_dir(directory)? {
        capture_path(root, &child?.path(), entries)?;
    }
    Ok(())
}

fn capture_path(
    root: &Path,
    path: &Path,
    entries: &mut BTreeMap<PathBuf, Entry>,
) -> io::Result<()> {
    let metadata = path.symlink_metadata()?;
    let kind = metadata.file_type();
    let mode = metadata.permissions().mode() & 0o7777;
    let entry = if kind.is_symlink() {
        Entry::Symlink(fs::read_link(path)?)
    } else if kind.is_dir() {
        Entry::Directory(mode)
    } else if kind.is_file() {
        let mut file = fs::File::open(path)?;
        let mut hash = blake3::Hasher::new();
        hash.update_reader(&mut file)?;
        Entry::File(mode, hash.finalize())
    } else {
        Entry::Other(mode)
    };
    entries.insert(
        path.strip_prefix(root)
            .expect("factory path stays under root")
            .to_path_buf(),
        entry,
    );
    if kind.is_dir() {
        capture_children(root, path, entries)?;
    }
    Ok(())
}

fn remove(path: &Path, kind: fs::FileType) -> io::Result<()> {
    if kind.is_dir() && !kind.is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

#[cfg(test)]
#[path = "factory_tests.rs"]
mod tests;
