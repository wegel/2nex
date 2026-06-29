//! Deterministic checksum calculation for package output directories.

use std::fs;
use std::io::{self, Read};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Calculate a deterministic checksum for every directory, regular file, or symlink.
pub fn calculate_output_checksum(output_dir: &Path) -> io::Result<String> {
    let mut entries = output_entries(output_dir)?;
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let mut final_hasher = blake3::Hasher::new();
    for entry in entries {
        final_hasher.update(entry.relative_path.as_os_str().as_bytes());
        final_hasher.update(b"\0");
        match entry.kind {
            OutputEntryKind::Directory(mode) => {
                final_hasher.update(b"directory");
                final_hasher.update(b"\0");
                final_hasher.update(&mode.to_le_bytes());
            }
            OutputEntryKind::File { mode, hash } => {
                final_hasher.update(b"file");
                final_hasher.update(b"\0");
                final_hasher.update(&mode.to_le_bytes());
                final_hasher.update(b"\0");
                final_hasher.update(&hash);
            }
            OutputEntryKind::Symlink { mode, target } => {
                final_hasher.update(b"symlink");
                final_hasher.update(b"\0");
                final_hasher.update(&mode.to_le_bytes());
                final_hasher.update(b"\0");
                final_hasher.update(&target);
            }
        }
        final_hasher.update(b"\0");
    }

    Ok(final_hasher.finalize().to_hex().to_string())
}

struct OutputEntry {
    relative_path: PathBuf,
    kind: OutputEntryKind,
}

enum OutputEntryKind {
    Directory(u32),
    File { mode: u32, hash: [u8; 32] },
    Symlink { mode: u32, target: Vec<u8> },
}

fn output_entries(output_dir: &Path) -> io::Result<Vec<OutputEntry>> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(output_dir) {
        let entry = entry?;
        let metadata = entry.path().symlink_metadata()?;
        let relative_path = entry
            .path()
            .strip_prefix(output_dir)
            .map_err(io::Error::other)?
            .to_path_buf();
        if entry.file_type().is_dir() {
            entries.push(OutputEntry {
                relative_path,
                kind: OutputEntryKind::Directory(metadata.mode()),
            });
        } else if entry.file_type().is_file() {
            entries.push(OutputEntry {
                relative_path,
                kind: OutputEntryKind::File {
                    mode: metadata.mode(),
                    hash: file_hash(entry.path())?,
                },
            });
        } else if entry.file_type().is_symlink() {
            entries.push(OutputEntry {
                relative_path,
                kind: OutputEntryKind::Symlink {
                    mode: metadata.mode(),
                    target: symlink_target_bytes(entry.path())?,
                },
            });
        }
    }
    Ok(entries)
}

fn file_hash(path: &Path) -> io::Result<[u8; 32]> {
    let mut hasher = blake3::Hasher::new();
    let mut file = fs::File::open(path)?;
    let mut buffer = [0u8; 65536];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(*hasher.finalize().as_bytes())
}

fn symlink_target_bytes(path: &Path) -> io::Result<Vec<u8>> {
    Ok(fs::read_link(path)?.as_os_str().as_bytes().to_vec())
}

#[cfg(test)]
#[path = "checksum_tests.rs"]
mod checksum_tests;
