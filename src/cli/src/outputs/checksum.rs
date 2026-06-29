//! Deterministic checksum calculation for package output directories.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Calculate a deterministic checksum for every regular file in an output.
pub fn calculate_output_checksum(output_dir: &Path) -> io::Result<String> {
    let mut file_paths = regular_file_paths(output_dir)?;
    file_paths.sort();

    let mut final_hasher = blake3::Hasher::new();
    for path in &file_paths {
        let relative_path = path
            .strip_prefix(output_dir)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let hash = file_hash(path)?;

        final_hasher.update(relative_path.as_bytes());
        final_hasher.update(b"\0");
        final_hasher.update(&hash);
        final_hasher.update(b"\0");
    }

    Ok(final_hasher.finalize().to_hex().to_string())
}

fn regular_file_paths(output_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut file_paths = Vec::new();
    for entry in WalkDir::new(output_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            file_paths.push(entry.path().to_path_buf());
        }
    }
    Ok(file_paths)
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
