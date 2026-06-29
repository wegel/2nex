//! Store metadata helpers for materializer checkouts.

use std::io;
use std::path::PathBuf;

use crate::store::Store;

/// Package identity for grouping outputs during materialization.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) struct PackageId {
    /// Package path, such as `core/init/systemd-with-dbus/257.5`.
    pub path: String,
    /// Manifest hash shared by all outputs from one package build.
    pub manifest_hash: String,
}

/// Get the package identity for a commit.
pub(super) fn get_package_id(
    repo_path: &str,
    commit: &str,
    fallback_repos: &[PathBuf],
) -> io::Result<PackageId> {
    let manifest_hash = get_manifest_hash(repo_path, commit, fallback_repos)?;
    let path = get_package_path(repo_path, commit, fallback_repos)?;
    Ok(PackageId {
        path,
        manifest_hash,
    })
}

fn get_commit_short_hash(
    repo_path: &str,
    commit: &str,
    fallback_repos: &[PathBuf],
) -> io::Result<String> {
    let store = Store::open_with_fallback_chain(repo_path, fallback_repos)?;
    let commit_id = store.resolve_ref(commit)?;
    Ok(commit_id[..8.min(commit_id.len())].to_string())
}

fn get_manifest_hash(
    repo_path: &str,
    commit: &str,
    fallback_repos: &[PathBuf],
) -> io::Result<String> {
    let store = Store::open_with_fallback_chain(repo_path, fallback_repos)?;
    match store.get_metadata(commit, "nex.manifest.hash")? {
        Some(hash) if !hash.is_empty() => Ok(hash),
        _ => get_commit_short_hash(repo_path, commit, fallback_repos),
    }
}

fn get_package_path(
    repo_path: &str,
    commit: &str,
    fallback_repos: &[PathBuf],
) -> io::Result<String> {
    let store = Store::open_with_fallback_chain(repo_path, fallback_repos)?;
    match store.get_metadata(commit, "nex.ref-binding")? {
        Some(binding) => {
            let ref_str = binding
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim_matches('\'')
                .trim_matches('"');
            parse_package_path_from_ref(ref_str)
        }
        None => parse_package_path_from_ref(commit),
    }
}

fn parse_package_path_from_ref(ref_str: &str) -> io::Result<String> {
    let parts: Vec<&str> = ref_str.split('/').collect();
    let Some(pkg_idx) = parts.iter().position(|&part| part == "pkg") else {
        return Ok(ref_str.replace('/', "_"));
    };
    let end_idx = parts
        .iter()
        .position(|&part| part == "outputs" || part == "bundles")
        .unwrap_or(parts.len());

    if end_idx > pkg_idx + 1 {
        Ok(parts[pkg_idx + 1..end_idx].join("/"))
    } else {
        Ok(ref_str.replace('/', "_"))
    }
}
