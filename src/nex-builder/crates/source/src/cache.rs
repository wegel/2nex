//! Locked, checksum-addressed cache writes.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::Path;

use fs2::FileExt;
use sha2::{Digest, Sha256};

pub(crate) fn materialize(
    output: &Path,
    expected: &str,
    generate: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    let parent = output
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "cache output has no parent"))?;
    fs::create_dir_all(parent)?;
    let lock = lock(output)?;
    lock.lock_exclusive()?;
    if output.is_file() && verify(output, expected).is_ok() {
        return Ok(());
    }
    if output.exists() {
        fs::remove_file(output)?;
    }
    let work = tempfile::Builder::new()
        .prefix(".nex-source-")
        .tempdir_in(parent)?;
    let temporary = work.path().join("output");
    generate(&temporary)?;
    let actual = sha256(&temporary)?;
    if actual != expected {
        let debug = debug_path(output, expected, &actual)?;
        if !debug.exists() {
            fs::rename(&temporary, &debug)?;
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "source checksum mismatch: expected {expected}, got {actual}; saved {}",
                debug.display()
            ),
        ));
    }
    fs::rename(temporary, output)
}

pub(crate) fn verify(path: &Path, expected: &str) -> io::Result<()> {
    let actual = sha256(path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "source checksum mismatch for {}: expected {expected}, got {actual}",
                path.display()
            ),
        ))
    }
}

pub(crate) fn sha256(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn locked<T>(target: &Path, action: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
    let lock = lock(target)?;
    lock.lock_exclusive()?;
    action()
}

fn lock(target: &Path) -> io::Result<File> {
    let parent = target
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "cache target has no parent"))?;
    fs::create_dir_all(parent)?;
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "cache name is not UTF-8"))?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(parent.join(format!(".{name}.lock")))
}

fn debug_path(output: &Path, expected: &str, actual: &str) -> io::Result<std::path::PathBuf> {
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "cache name is not UTF-8"))?;
    Ok(output.with_file_name(name.replacen(expected, actual, 1)))
}
