//! Files-ref derivation for capsule flattening.

use std::path::PathBuf;

use crate::manifest::types::Manifest;
use crate::utils::hash_file_content;

#[cfg(test)]
#[path = "flatten_refs_tests.rs"]
mod flatten_refs_tests;

/// Derive the files commit for the current manifest.
///
/// Stable packages use the manifest checksum. Bootstrap packages use the
/// manifest blob hash, so the files ref changes when the manifest changes.
pub(super) fn derive_files_commit_for_manifest(manifest: &Manifest) -> Option<String> {
    let has_stable_checksum =
        manifest.package.checksum.is_some() && manifest.package.stable_checksum.unwrap_or(true);

    let address_hash = if has_stable_checksum {
        manifest.package.checksum.clone()?
    } else {
        let manifest_path = PathBuf::from(format!(
            "{}/{}.yaml",
            manifest.package.namespace_path(),
            manifest.package.slug
        ));
        hash_file_content(&manifest_path).ok()?
    };

    Some(format!(
        "x86_64/{}/{}/{}/{}/files",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version,
        address_hash
    ))
}
