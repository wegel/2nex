//! Manifest and store-ref lookup helpers for build orchestration.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::types::Manifest;
use crate::refs::PackageRef;
use crate::store::{find_commit_by_manifest_hash, lookup_artifact};

/// Check if a build output exists for a manifest hash.
pub fn build_exists_for_manifest(
    repo_path: &str,
    commit_ref: &str,
    manifest_hash: &str,
) -> io::Result<bool> {
    if let Ok(pkg_ref) = PackageRef::parse(commit_ref) {
        let artifact_path = pkg_ref.artifact_ref_path(manifest_hash);
        if let Ok(Some(_tree)) = lookup_artifact(repo_path, &artifact_path) {
            return Ok(true);
        }
    }

    match find_commit_by_manifest_hash(repo_path, commit_ref, manifest_hash) {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Return the build directory used for a package in parallel builds.
pub fn get_build_dir_for_package(manifest: &Manifest) -> String {
    format!(
        ".nex/tmp/build_rootfs_{}_{}",
        manifest.package.slug.replace("/", "_"),
        manifest.package.namespace.replace("/", "_")
    )
}

/// Find the package manifest file that corresponds to a store commit ref.
pub fn find_manifest_for_commit(commit: &str, manifest_dirs: &[PathBuf]) -> io::Result<PathBuf> {
    let pkg_ref = PackageRef::parse(commit).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid commit reference format: {} ({})", commit, e),
        )
    })?;

    let mut matches = BTreeSet::new();
    for base_dir in manifest_dirs {
        for path in find_manifests_in_base_dir(&pkg_ref, base_dir) {
            matches.insert(path.canonicalize()?);
        }
    }

    if matches.len() == 1 {
        return Ok(matches.pop_first().expect("one manifest match"));
    }
    if matches.len() > 1 {
        let paths = matches
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("duplicate manifests for {}: {}", commit, paths),
        ));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "Could not find manifest for commit {} (slug: {}, namespace: {})",
            commit, pkg_ref.slug, pkg_ref.namespace
        ),
    ))
}

fn find_manifests_in_base_dir(pkg_ref: &PackageRef, base_dir: &Path) -> Vec<PathBuf> {
    let mut matches = BTreeSet::new();
    for namespace_path in namespace_search_paths(pkg_ref, base_dir) {
        let direct_path = namespace_path.join(format!("{}.yaml", pkg_ref.slug));
        if direct_path.exists() {
            matches.insert(direct_path);
        }

        for path in manifest_paths(&namespace_path) {
            if is_yaml_with_slug_suffix(&path, &pkg_ref.slug)
                || declares_package_slug(&path, &pkg_ref.slug)
            {
                matches.insert(path);
            }
        }
    }
    matches.into_iter().collect()
}

fn namespace_search_paths(pkg_ref: &PackageRef, base_dir: &Path) -> Vec<PathBuf> {
    let mut paths = vec![
        base_dir.join("pkg").join(&pkg_ref.namespace),
        base_dir.join(&pkg_ref.namespace),
    ];
    if base_dir.ends_with("pkg") {
        paths.push(base_dir.join(&pkg_ref.namespace));
    }
    paths
}

fn manifest_paths(namespace_path: &Path) -> Vec<PathBuf> {
    if !namespace_path.is_dir() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(namespace_path) else {
        return Vec::new();
    };
    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_yaml(path))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn is_yaml_with_slug_suffix(path: &Path, slug: &str) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|filename| filename.ends_with(&format!("-{}", slug)))
}

fn is_yaml(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("yaml" | "yml")
    )
}

fn declares_package_slug(path: &Path, slug: &str) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(document) = serde_yaml::from_str::<serde_yaml::Value>(&contents) else {
        return false;
    };

    document
        .get("package")
        .and_then(|package| package.get("slug"))
        .and_then(serde_yaml::Value::as_str)
        == Some(slug)
}

#[cfg(test)]
#[path = "manifest_lookup_tests.rs"]
mod tests;
