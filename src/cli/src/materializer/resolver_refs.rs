//! Convert dependency names into checksum-addressed files refs.

use std::path::PathBuf;

use crate::manifest::types::Manifest;
use crate::manifest::ManifestIndex;
use crate::store::Store;
use crate::utils::hash_file_content;

/// Result of resolving a dependency to a commit.
pub(super) enum ResolveResult {
    /// Successfully resolved to a files commit.
    Ok(String),
    /// Dependency not found in manifest.
    NotFound,
    /// Manifest not found for dependency.
    ManifestNotFound(String),
    /// Files commit doesn't exist and needs `nex compute-deps`.
    FilesCommitMissing { package: String, checksum: String },
}

pub(super) fn manifest_files_ref(manifest: &Manifest) -> Option<String> {
    let package_ref = ManifestPackageRef {
        namespace_path: manifest.package.namespace_path(),
        slug: manifest.package.slug.clone(),
        version: manifest.package.version.clone(),
    };
    let address_hash = manifest_address_hash(manifest, &package_ref)?;
    Some(package_ref.files_ref(&address_hash))
}

/// Resolve a dependency name to a checksum-addressed files commit.
pub(super) fn resolve_dependency_to_commit(
    dep_name: &str,
    source_manifest: &crate::manifest::types::Manifest,
    manifest_index: &ManifestIndex,
    store: &Store,
) -> ResolveResult {
    let dep = match source_manifest
        .dependencies
        .iter()
        .find(|dep| dep.name.as_deref() == Some(dep_name))
    {
        Some(dep) => dep,
        None => return ResolveResult::NotFound,
    };

    let Some(package_ref) = dependency_package_ref(&dep.commit) else {
        return ResolveResult::NotFound;
    };
    let dep_manifest =
        match manifest_index.get_manifest(&package_ref.namespace_path, &package_ref.slug) {
            Some(manifest) => manifest,
            None => return ResolveResult::ManifestNotFound(package_ref.package_path),
        };

    let address_hash = match dependency_address_hash(dep_manifest, &package_ref) {
        Some(hash) => hash,
        None => return ResolveResult::NotFound,
    };
    let files_ref = package_ref.files_ref(&address_hash);
    if store.resolve_ref(&files_ref).is_ok() {
        return ResolveResult::Ok(files_ref);
    }

    ResolveResult::FilesCommitMissing {
        package: package_ref.package_path,
        checksum: address_hash,
    }
}

struct DependencyPackageRef {
    namespace_path: String,
    slug: String,
    version: String,
    package_path: String,
}

struct ManifestPackageRef {
    namespace_path: String,
    slug: String,
    version: String,
}

impl DependencyPackageRef {
    fn files_ref(&self, address_hash: &str) -> String {
        format!(
            "x86_64/{}/{}/{}/{}/files",
            self.namespace_path, self.slug, self.version, address_hash
        )
    }
}

impl ManifestPackageRef {
    fn files_ref(&self, address_hash: &str) -> String {
        format!(
            "x86_64/{}/{}/{}/{}/files",
            self.namespace_path, self.slug, self.version, address_hash
        )
    }
}

fn dependency_package_ref(commit: &str) -> Option<DependencyPackageRef> {
    let parts: Vec<&str> = commit.split('/').collect();
    let pkg_idx = parts.iter().position(|&part| part == "pkg")?;
    let end_idx = parts
        .iter()
        .position(|&part| part == "outputs" || part == "bundles")?;
    if end_idx <= pkg_idx + 2 {
        return None;
    }

    let slug = parts[end_idx - 2].to_string();
    let namespace_path = parts[pkg_idx..end_idx - 2].join("/");
    Some(DependencyPackageRef {
        package_path: format!("{}/{}", namespace_path, slug),
        namespace_path,
        slug,
        version: parts[end_idx - 1].to_string(),
    })
}

fn dependency_address_hash(
    manifest: &crate::manifest::types::Manifest,
    package_ref: &DependencyPackageRef,
) -> Option<String> {
    let has_stable_checksum =
        manifest.package.checksum.is_some() && manifest.package.stable_checksum.unwrap_or(true);
    if has_stable_checksum {
        return manifest.package.checksum.clone();
    }

    let manifest_path = PathBuf::from(format!(
        "{}/{}.yaml",
        package_ref.namespace_path, package_ref.slug
    ));
    hash_file_content(&manifest_path).ok()
}

fn manifest_address_hash(manifest: &Manifest, package_ref: &ManifestPackageRef) -> Option<String> {
    let has_stable_checksum =
        manifest.package.checksum.is_some() && manifest.package.stable_checksum.unwrap_or(true);
    if has_stable_checksum {
        return manifest.package.checksum.clone();
    }

    let manifest_path = PathBuf::from(format!(
        "{}/{}.yaml",
        package_ref.namespace_path, package_ref.slug
    ));
    hash_file_content(&manifest_path).ok()
}
