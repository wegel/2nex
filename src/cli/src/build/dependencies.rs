//! Dependency commit resolution for package builds.

use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::manifest::Dependency;
use crate::store::find_commit_by_manifest_hash;

use super::orchestration::find_manifest_for_commit;

/// Resolve dependency refs to concrete commits when the manifest pins a blob.
pub fn resolve_dependency_commits(
    dependencies: &[Dependency],
    repo_path: &str,
    manifest_dirs: &[PathBuf],
) -> io::Result<Vec<String>> {
    let mut resolved = Vec::new();

    for dep in dependencies {
        resolved.push(resolve_dependency_commit(dep, repo_path, manifest_dirs));
    }

    Ok(resolved)
}

fn resolve_dependency_commit(
    dep: &Dependency,
    repo_path: &str,
    manifest_dirs: &[PathBuf],
) -> String {
    let Some(blob_sha) = dep.manifest_ref.as_ref() else {
        return dep.commit.clone();
    };

    let git_root = match dependency_git_root(dep, manifest_dirs) {
        Ok(root) => root,
        Err(error) => {
            eprintln!(
                "Warning: failed to locate manifest for {}: {}",
                dep.commit, error
            );
            return dep.commit.clone();
        }
    };

    match matching_dependency_commit(dep, repo_path, &git_root, blob_sha) {
        Some(commit_id) => commit_id,
        None => dep.commit.clone(),
    }
}

fn dependency_git_root(dep: &Dependency, manifest_dirs: &[PathBuf]) -> io::Result<PathBuf> {
    let manifest_path = find_manifest_for_commit(&dep.commit, manifest_dirs)?;
    crate::manifest::repository_root_for_path(&manifest_path)
}

fn matching_dependency_commit(
    dep: &Dependency,
    repo_path: &str,
    git_root: &Path,
    blob_sha: &str,
) -> Option<String> {
    let content = match crate::utils::fetch_git_blob(git_root, blob_sha) {
        Ok(content) => content,
        Err(e) => {
            eprintln!(
                "Warning: failed to fetch blob {} for {}: {}",
                blob_sha, dep.commit, e
            );
            return None;
        }
    };
    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));

    match find_commit_by_manifest_hash(repo_path, &dep.commit, &content_hash) {
        Ok(Some(commit_id)) => Some(commit_id),
        Ok(None) => {
            eprintln!(
                "Warning: no matching commit found for {} with manifest_ref {}",
                dep.commit, blob_sha
            );
            None
        }
        Err(e) => {
            eprintln!(
                "Warning: failed to search history for {}: {}",
                dep.commit, e
            );
            None
        }
    }
}
