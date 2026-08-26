//! Reproducible tar and filesystem helpers.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::{Builder, EntryType, Header};

const MTIME: u64 = 1_704_067_200;

pub(crate) fn create(source: &Path, output: &Path) -> io::Result<()> {
    let parent = source.parent().unwrap_or(source);
    let encoder = GzEncoder::new(File::create(output)?, Compression::best());
    let mut archive = Builder::new(encoder);
    append(&mut archive, source, relative(source, parent)?)?;
    append_below(&mut archive, source, parent)?;
    let encoder = archive.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn append_below(
    archive: &mut Builder<GzEncoder<File>>,
    directory: &Path,
    parent: &Path,
) -> io::Result<()> {
    let mut entries = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    for path in entries {
        let metadata = fs::symlink_metadata(&path)?;
        append(archive, &path, relative(&path, parent)?)?;
        if metadata.is_dir() {
            append_below(archive, &path, parent)?;
        }
    }
    Ok(())
}

fn relative<'a>(path: &'a Path, parent: &Path) -> io::Result<&'a Path> {
    path.strip_prefix(parent).map_err(io::Error::other)
}

fn append(archive: &mut Builder<GzEncoder<File>>, path: &Path, relative: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    let mut header = Header::new_gnu();
    header.set_mtime(MTIME);
    header.set_uid(0);
    header.set_gid(0);
    if metadata.is_dir() {
        header.set_entry_type(EntryType::Directory);
        header.set_size(0);
        header.set_mode(0o755);
        archive.append_data(&mut header, relative, io::empty())
    } else if metadata.file_type().is_symlink() {
        header.set_entry_type(EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        archive.append_link(&mut header, relative, fs::read_link(path)?)
    } else {
        header.set_entry_type(EntryType::Regular);
        header.set_size(metadata.len());
        header.set_mode(if metadata.permissions().mode() & 0o111 == 0 {
            0o644
        } else {
            0o755
        });
        archive.append_data(&mut header, relative, File::open(path)?)
    }
}

pub(crate) fn copy_tree(source: &Path, destination: &Path) -> io::Result<()> {
    copy_tree_inner(source, destination, &mut BTreeSet::new())
}

fn copy_tree_inner(
    source: &Path,
    destination: &Path,
    ancestors: &mut BTreeSet<PathBuf>,
) -> io::Result<()> {
    let canonical = fs::canonicalize(source)?;
    if !ancestors.insert(canonical.clone()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("directory link forms a cycle at {}", source.display()),
        ));
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source = entry.path();
        let destination = destination.join(entry.file_name());
        if source.is_dir() {
            copy_tree_inner(&source, &destination, ancestors)?;
        } else {
            fs::copy(source, destination)?;
        }
    }
    ancestors.remove(&canonical);
    Ok(())
}

pub(crate) fn extract_tar(archive: &Path, destination: &Path, strip: bool) -> io::Result<()> {
    if !strip {
        fs::create_dir_all(destination)?;
        return unpack_tar(archive, destination);
    }

    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no parent directory", destination.display()),
        )
    })?;
    fs::create_dir_all(parent)?;
    let temporary = tempfile::Builder::new()
        .prefix(".untar-")
        .tempdir_in(parent)?;
    unpack_tar(archive, temporary.path())?;
    let root = only_directory(temporary.path())?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} does not contain one root directory", archive.display()),
        )
    })?;
    fs::rename(root, destination)
}

pub(crate) fn extract_zip(archive: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    let mut archive = zip::ZipArchive::new(File::open(archive)?).map_err(io::Error::other)?;
    archive.extract(destination).map_err(io::Error::other)
}

fn unpack_tar(path: &Path, destination: &Path) -> io::Result<()> {
    let mut file = File::open(path)?;
    let mut magic = [0; 2];
    let gzip = file.read(&mut magic)? == magic.len() && magic == [0x1f, 0x8b];
    drop(file);

    if gzip {
        tar::Archive::new(GzDecoder::new(File::open(path)?)).unpack(destination)
    } else {
        tar::Archive::new(File::open(path)?).unpack(destination)
    }
}

pub(crate) fn only_directory(root: &Path) -> io::Result<Option<PathBuf>> {
    let mut entries = fs::read_dir(root)?;
    let first = match entries.next() {
        Some(entry) => entry?,
        None => return Ok(None),
    };
    if entries.next().transpose()?.is_some() || !first.path().is_dir() {
        return Ok(None);
    }
    Ok(Some(first.path()))
}

#[cfg(test)]
#[path = "archive_tests.rs"]
mod tests;
