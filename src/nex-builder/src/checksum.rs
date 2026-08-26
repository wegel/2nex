//! Deterministic checksums for package output trees.

use std::fs;
use std::io::{self, Read};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use walkdir::WalkDir;

pub(crate) fn output_checksum(root: &Path) -> io::Result<String> {
    let mut hasher = blake3::Hasher::new();
    for entry in WalkDir::new(root).sort_by_file_name() {
        let entry = entry?;
        hash_entry(&mut hasher, root, entry.path())?;
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn hash_entry(hasher: &mut blake3::Hasher, root: &Path, path: &Path) -> io::Result<()> {
    let relative = path.strip_prefix(root).map_err(io::Error::other)?;
    let metadata = path.symlink_metadata()?;
    hasher.update(relative.as_os_str().as_bytes());
    hasher.update(b"\0");

    if metadata.is_dir() {
        hasher.update(b"directory\0");
        hasher.update(&metadata.mode().to_le_bytes());
    } else if metadata.is_file() {
        hasher.update(b"file\0");
        hasher.update(&metadata.mode().to_le_bytes());
        hasher.update(b"\0");
        hasher.update(&file_hash(path)?);
    } else if metadata.file_type().is_symlink() {
        hasher.update(b"symlink\0");
        hasher.update(&metadata.mode().to_le_bytes());
        hasher.update(b"\0");
        hasher.update(fs::read_link(path)?.as_os_str().as_bytes());
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported output entry {}", path.display()),
        ));
    }
    hasher.update(b"\0");
    Ok(())
}

fn file_hash(path: &Path) -> io::Result<[u8; 32]> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok(*hasher.finalize().as_bytes());
        }
        hasher.update(&buffer[..count]);
    }
}
