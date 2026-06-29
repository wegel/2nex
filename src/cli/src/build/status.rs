//! Store status checks for already-built package outputs.

use sha2::{Digest, Sha256};

use std::fs;
use std::io;
use std::path::Path;

use crate::manifest::Manifest;
use crate::store::{ensure_branch_exists, find_commit_by_manifest_hash, lookup_artifact};

/// Compute the SHA-256 hash of the manifest file bytes.
pub fn compute_manifest_hash(manifest_path: &Path) -> io::Result<String> {
    let contents = fs::read(manifest_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Return a store commit marker when all manifest outputs match the current manifest hash.
pub fn check_if_built(
    repo_path: &str,
    manifest: &Manifest,
    manifest_path: &Path,
) -> io::Result<Option<String>> {
    let current_hash = compute_manifest_hash(manifest_path)?;
    let mut found_commit: Option<String> = None;

    for output_name in manifest.outputs.keys() {
        let branch = output_branch_name(manifest, output_name);
        let artifact_path = output_artifact_path(manifest, &current_hash, output_name);

        if let Ok(Some(_tree)) = lookup_artifact(repo_path, &artifact_path) {
            found_commit.get_or_insert_with(|| "artifact".to_string());
            continue;
        }

        if ensure_branch_exists(repo_path, &branch).is_err() {
            return Ok(None);
        }

        match find_commit_by_manifest_hash(repo_path, &branch, &current_hash)? {
            Some(commit_id) => {
                found_commit.get_or_insert(commit_id);
            }
            None => return Ok(None),
        }
    }

    Ok(found_commit)
}

pub(super) fn output_branch_name(manifest: &Manifest, output_type: &str) -> String {
    format!(
        "x86_64/{}/{}/{}/outputs/{}",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version,
        output_type
    )
}

pub(super) fn output_artifact_path(
    manifest: &Manifest,
    manifest_hash: &str,
    output_type: &str,
) -> String {
    format!(
        "x86_64/{}/{}/{}/{}/outputs/{}",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version,
        manifest_hash,
        output_type
    )
}
