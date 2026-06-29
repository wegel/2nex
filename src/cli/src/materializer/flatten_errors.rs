//! Error helpers for runtime dependency flattening.

use std::io;

use crate::manifest::types::{Dependency, Manifest};

pub(super) fn flatten_export_error(commit: &str, path: &str, error: io::Error) -> io::Error {
    io::Error::new(
        error.kind(),
        format!(
            "failed to flatten declared runtime file {} from {}: {}",
            path, commit, error
        ),
    )
}

pub(super) fn missing_manifest_error(commit: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "cannot flatten runtime dependencies for {} because no manifest was found",
            commit
        ),
    )
}

pub(super) fn missing_bundle_error(bundle_name: &str, manifest: &Manifest) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "bundle '{}' is referenced for {}/{} but is not declared",
            bundle_name, manifest.package.namespace, manifest.package.slug
        ),
    )
}

pub(super) fn missing_output_error(output_name: &str, manifest: &Manifest) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "output '{}' is referenced for {}/{} but is not declared",
            output_name, manifest.package.namespace, manifest.package.slug
        ),
    )
}

pub(super) fn missing_self_files_commit_error(
    manifest: &Manifest,
    self_libs: &[String],
) -> io::Error {
    let paths = self_libs.join(", ");
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "cannot flatten self runtime files for {}/{} because no files commit can be derived; declared file(s): {}",
            manifest.package.namespace, manifest.package.slug, paths
        ),
    )
}

pub(super) fn missing_dependency_error(
    file_path: &str,
    dep_name: &str,
    manifest: &Manifest,
) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "{} resolves to dependency '{}' in {}/{} but that dependency is not declared",
            file_path, dep_name, manifest.package.namespace, manifest.package.slug
        ),
    )
}

pub(super) fn missing_dependency_files_commit_error(
    file_path: &str,
    dep_name: &str,
    dep: &Dependency,
) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "{} resolves to dependency '{}' at {} but no files commit can be derived",
            file_path, dep_name, dep.commit
        ),
    )
}

pub(super) fn missing_resolution_error(file_path: &str, manifest: &Manifest) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "{} is listed as a runtime need in {}/{} but has no resolution entry",
            file_path, manifest.package.namespace, manifest.package.slug
        ),
    )
}
