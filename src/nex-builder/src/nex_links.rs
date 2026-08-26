//! Public filesystem links backed by files inside Nex package capsules.

use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub(crate) fn link_capsule(physical: &Path, logical: &Path, target: &Path) -> io::Result<()> {
    for entry in WalkDir::new(physical).min_depth(1).sort_by_file_name() {
        let entry = entry?;
        let relative = entry
            .path()
            .strip_prefix(physical)
            .expect("walk stays in capsule");
        if relative == Path::new(".nex-app-root") {
            continue;
        }
        let destination = target.join(relative);
        if entry.file_type().is_dir() {
            create_directory(&destination)?;
        } else if destination.symlink_metadata().is_err() {
            symlink(logical.join(relative), destination)?;
        }
    }
    Ok(())
}

pub(crate) fn link_libraries(package_root: &Path, target: &Path) -> io::Result<()> {
    let mut libraries = Vec::new();
    for entry in WalkDir::new(package_root).min_depth(1) {
        let entry = entry?;
        if entry.file_type().is_dir() && entry.path().ends_with("usr/lib") {
            libraries.push(entry.into_path());
        }
    }
    libraries.sort();
    let public = target.join("usr/lib");
    fs::create_dir_all(&public)?;
    for directory in libraries {
        link_library_directory(package_root, &public, &directory)?;
    }
    Ok(())
}

pub(crate) fn create_fhs_links(target: &Path) -> io::Result<()> {
    for (name, destination) in [("bin", "usr/bin"), ("sbin", "usr/bin"), ("lib", "usr/lib")] {
        let path = target.join(name);
        if path.symlink_metadata().is_err() {
            symlink(destination, path)?;
        }
    }
    fs::create_dir_all(target.join("usr/lib"))?;
    let sbin = target.join("usr/sbin");
    if sbin.symlink_metadata().is_err() {
        symlink("bin", sbin)?;
    }
    Ok(())
}

fn create_directory(path: &Path) -> io::Result<()> {
    match path.symlink_metadata() {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("package directory conflicts with {}", path.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(path),
        Err(error) => Err(error),
    }
}

fn link_library_directory(package_root: &Path, public: &Path, directory: &Path) -> io::Result<()> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    let loaders = entries
        .iter()
        .filter(|entry| is_loader(&entry.file_name().to_string_lossy()))
        .map(fs::DirEntry::path)
        .collect::<Vec<_>>();
    for entry in entries {
        if !is_loader(&entry.file_name().to_string_lossy()) {
            link_library(package_root, public, &entry.path(), &loaders)?;
        }
    }
    Ok(())
}

fn link_library(
    package_root: &Path,
    public: &Path,
    source: &Path,
    loaders: &[PathBuf],
) -> io::Result<()> {
    let name = source.file_name().expect("library has name");
    let destination = public.join(name);
    if copy_linker_script(package_root, public, source, &destination, loaders)? {
        return Ok(());
    }
    if destination.symlink_metadata().is_ok() {
        return Ok(());
    }
    let relative = source
        .strip_prefix(package_root)
        .expect("library is in package root");
    symlink(Path::new("../../nex/pkg").join(relative), destination)
}

fn copy_linker_script(
    package_root: &Path,
    public: &Path,
    source: &Path,
    destination: &Path,
    loaders: &[PathBuf],
) -> io::Result<bool> {
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.is_file() || metadata.len() > 64 * 1024 {
        return Ok(false);
    }
    let Ok(mut content) = fs::read_to_string(source) else {
        return Ok(false);
    };
    let mut changed = false;
    for loader in loaders {
        let name = loader
            .file_name()
            .and_then(|value| value.to_str())
            .expect("loader has UTF-8 name");
        let original = format!("/usr/lib/{name}");
        if content.contains(&original) {
            content = content.replace(&original, &format!("/usr/lib/nex-linker/{name}"));
            link_loader(package_root, public, loader)?;
            changed = true;
        }
    }
    if changed {
        if destination.symlink_metadata().is_ok() {
            fs::remove_file(destination)?;
        }
        fs::write(destination, content)?;
        fs::set_permissions(destination, metadata.permissions())?;
    }
    Ok(changed)
}

fn link_loader(package_root: &Path, public: &Path, loader: &Path) -> io::Result<()> {
    let directory = public.join("nex-linker");
    fs::create_dir_all(&directory)?;
    let destination = directory.join(loader.file_name().expect("loader has name"));
    if destination.symlink_metadata().is_ok() {
        return Ok(());
    }
    let relative = loader
        .strip_prefix(package_root)
        .expect("loader is in package root");
    symlink(Path::new("../../../nex/pkg").join(relative), destination)
}

fn is_loader(name: &str) -> bool {
    name.starts_with("ld-linux") || name == "ld.so"
}
