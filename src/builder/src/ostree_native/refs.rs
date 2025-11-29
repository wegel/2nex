//! OSTree ref listing and resolution.

use std::fs;
use std::io;
use std::path::Path;

/// list all refs matching an optional pattern.
/// refs are plain text files under refs/heads/ containing commit checksums.
pub fn list_refs(repo_path: &Path, pattern: Option<&str>) -> io::Result<Vec<String>> {
    let refs_dir = repo_path.join("refs/heads");
    if !refs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut refs = Vec::new();
    walk_refs_dir(&refs_dir, &refs_dir, pattern, &mut refs)?;
    refs.sort();
    Ok(refs)
}

fn walk_refs_dir(
    base: &Path,
    current: &Path,
    pattern: Option<&str>,
    refs: &mut Vec<String>,
) -> io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            walk_refs_dir(base, &path, pattern, refs)?;
        } else {
            let rel_path = path.strip_prefix(base).unwrap();
            let ref_name = rel_path.to_string_lossy().to_string();

            if pattern.map_or(true, |p| ref_name.contains(p)) {
                refs.push(ref_name);
            }
        }
    }
    Ok(())
}

/// resolve a ref name to its commit checksum.
pub fn resolve_ref(repo_path: &Path, ref_name: &str) -> io::Result<String> {
    let ref_path = repo_path.join("refs/heads").join(ref_name);
    let content = fs::read_to_string(&ref_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("failed to resolve ref '{}': {}", ref_name, e),
        )
    })?;
    Ok(content.trim().to_string())
}

/// check if a string looks like a commit checksum (64 hex chars).
pub fn is_checksum(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// resolve a ref or checksum to a checksum.
pub fn resolve_ref_or_checksum(repo_path: &Path, ref_or_checksum: &str) -> io::Result<String> {
    if is_checksum(ref_or_checksum) {
        Ok(ref_or_checksum.to_string())
    } else {
        resolve_ref(repo_path, ref_or_checksum)
    }
}
