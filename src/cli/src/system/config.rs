//! Configuration layout for read-only Nex deployments.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
enum FactoryEntry {
    Directory(u32),
    File(u32, Vec<u8>),
    Symlink(PathBuf),
    Other(u32),
}

/// Factory defaults contributed by an immutable base before the child adds
/// packages, files, or script output.
#[derive(Clone, Debug, Default)]
pub(super) struct FactoryDefaultsSnapshot {
    entries: BTreeMap<PathBuf, FactoryEntry>,
}

pub(super) fn capture_factory_defaults(target: &Path) -> io::Result<FactoryDefaultsSnapshot> {
    let factory = target.join("usr/share/factory/etc");
    let mut snapshot = FactoryDefaultsSnapshot::default();
    if factory.exists() {
        capture_children(&factory, &factory, &mut snapshot.entries)?;
    }
    Ok(snapshot)
}

/// Move legacy package defaults out of `/etc` before a Nex deployment is stored.
///
/// The initramfs mounts persistent host state over `/etc` at boot. Programs that
/// do not yet follow UAPI.6 still need pristine defaults, so Nex keeps those
/// files under `/usr/share/factory/etc` and seeds only missing host paths.
pub(super) fn move_legacy_etc_to_factory(
    target: &Path,
    base_defaults: Option<&FactoryDefaultsSnapshot>,
) -> io::Result<()> {
    let etc = target.join("etc");
    let factory = target.join("usr/share/factory/etc");

    if !etc.exists() {
        fs::create_dir_all(&etc)?;
        return Ok(());
    }

    if factory.exists() {
        merge_defaults(&etc, &factory, &factory, base_defaults)?;
        fs::remove_dir(&etc)?;
    } else {
        let parent = factory
            .parent()
            .expect("factory etc path always has a parent");
        fs::create_dir_all(parent)?;
        fs::rename(&etc, &factory)?;
    }

    fs::create_dir(&etc)?;
    println!("  Moved legacy /etc defaults to /usr/share/factory/etc");
    Ok(())
}

fn merge_defaults(
    source: &Path,
    destination: &Path,
    factory_root: &Path,
    base_defaults: Option<&FactoryDefaultsSnapshot>,
) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        let source_type = entry.file_type()?;
        let destination_type = match destination_path.symlink_metadata() {
            Ok(metadata) => metadata.file_type(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::rename(&source_path, &destination_path)?;
                continue;
            }
            Err(error) => return Err(error),
        };
        if source_type.is_dir() && destination_type.is_dir() {
            merge_defaults(&source_path, &destination_path, factory_root, base_defaults)?;
            fs::remove_dir(&source_path)?;
            continue;
        }

        let replaces_unchanged_base = base_defaults
            .map(|snapshot| destination_matches_snapshot(&destination_path, factory_root, snapshot))
            .transpose()?
            .unwrap_or(false);
        if replaces_unchanged_base {
            remove_destination(&destination_path, destination_type)?;
            fs::rename(&source_path, &destination_path)?;
            continue;
        }

        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "legacy /etc default conflicts with native factory file: {}",
                destination_path.display()
            ),
        ));
    }
    Ok(())
}

fn destination_matches_snapshot(
    destination: &Path,
    factory_root: &Path,
    snapshot: &FactoryDefaultsSnapshot,
) -> io::Result<bool> {
    let relative = destination
        .strip_prefix(factory_root)
        .expect("factory destination stays below its root");
    let mut current = BTreeMap::new();
    capture_path(factory_root, destination, &mut current)?;
    let expected = snapshot
        .entries
        .iter()
        .filter(|(path, _)| path.as_path() == relative || path.starts_with(relative))
        .map(|(path, entry)| (path.clone(), entry.clone()))
        .collect::<BTreeMap<_, _>>();
    Ok(!expected.is_empty() && current == expected)
}

fn capture_children(
    root: &Path,
    directory: &Path,
    entries: &mut BTreeMap<PathBuf, FactoryEntry>,
) -> io::Result<()> {
    for child in fs::read_dir(directory)? {
        capture_path(root, &child?.path(), entries)?;
    }
    Ok(())
}

fn capture_path(
    root: &Path,
    path: &Path,
    entries: &mut BTreeMap<PathBuf, FactoryEntry>,
) -> io::Result<()> {
    let metadata = path.symlink_metadata()?;
    let file_type = metadata.file_type();
    let mode = metadata.permissions().mode() & 0o7777;
    let entry = if file_type.is_symlink() {
        FactoryEntry::Symlink(fs::read_link(path)?)
    } else if file_type.is_dir() {
        FactoryEntry::Directory(mode)
    } else if file_type.is_file() {
        FactoryEntry::File(mode, fs::read(path)?)
    } else {
        FactoryEntry::Other(mode)
    };
    entries.insert(
        path.strip_prefix(root)
            .expect("captured factory path stays below its root")
            .to_path_buf(),
        entry,
    );
    if file_type.is_dir() {
        capture_children(root, path, entries)?;
    }
    Ok(())
}

fn remove_destination(path: &Path, file_type: fs::FileType) -> io::Result<()> {
    if file_type.is_dir() && !file_type.is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
