//! Dependency commit resolution for package builds.

use std::io;
use std::path::PathBuf;

use crate::manifest::{compute_manifest_hash_from_source, Dependency, ManifestSource};
use crate::store::find_commit_by_manifest_hash;

use super::orchestration::find_manifest_for_commit;

/// Resolve dependency refs to concrete commits when the manifest pins a repository revision.
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
    let Some(revision) = dep.manifest_ref.as_ref() else {
        return dep.commit.clone();
    };

    let (manifest_path, git_root) = match dependency_manifest_location(dep, manifest_dirs) {
        Ok(location) => location,
        Err(error) => {
            eprintln!(
                "Warning: failed to locate manifest for {}: {}",
                dep.commit, error
            );
            return dep.commit.clone();
        }
    };

    match matching_dependency_commit(dep, repo_path, manifest_path, git_root, revision) {
        Some(commit_id) => commit_id,
        None => dep.commit.clone(),
    }
}

fn dependency_manifest_location(
    dep: &Dependency,
    manifest_dirs: &[PathBuf],
) -> io::Result<(PathBuf, PathBuf)> {
    let manifest_path = find_manifest_for_commit(&dep.commit, manifest_dirs)?;
    let git_root = crate::manifest::repository_root_for_path(&manifest_path)?;
    Ok((manifest_path, git_root))
}

fn matching_dependency_commit(
    dep: &Dependency,
    repo_path: &str,
    manifest_path: PathBuf,
    git_root: PathBuf,
    revision: &str,
) -> Option<String> {
    let source = ManifestSource::Repository {
        revision: revision.to_string(),
        path: manifest_path,
        git_root,
    };
    let content_hash = match compute_manifest_hash_from_source(&source) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!(
                "Warning: failed to fetch repository revision {} for {}: {}",
                revision, dep.commit, e
            );
            return None;
        }
    };

    match find_commit_by_manifest_hash(repo_path, &dep.commit, &content_hash) {
        Ok(Some(commit_id)) => Some(commit_id),
        Ok(None) => {
            eprintln!(
                "Warning: no matching commit found for {} with manifest_ref {}",
                dep.commit, revision
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
