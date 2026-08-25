//! Working-tree lookup for assembly base manifests.

use std::io;
use std::path::{Component, Path, PathBuf};

use super::repositories::NEX_REPOSITORY_PREFIX;
use super::{ManifestRepositories, ManifestSource, SystemBase};

/// Locate the live manifest that builds an assembly's base output.
pub fn system_base_source(base: &SystemBase, owner: &ManifestSource) -> io::Result<ManifestSource> {
    let repositories = ManifestRepositories::discover(owner.path())?;
    let (root, relative) = base_location(&base.manifest, &repositories)?;
    let path = root.join(relative).canonicalize().map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "cannot resolve base manifest {}: {}",
                base.manifest.display(),
                error
            ),
        )
    })?;
    if !path.starts_with(root) {
        return Err(invalid_base_path(&base.manifest));
    }
    Ok(ManifestSource::Path(path))
}

fn base_location<'a>(
    reference: &Path,
    repositories: &'a ManifestRepositories,
) -> io::Result<(&'a Path, PathBuf)> {
    let text = reference.to_string_lossy();
    let (root, relative) = match text.strip_prefix(NEX_REPOSITORY_PREFIX) {
        Some(path) => (
            repositories
                .upstream_root()
                .unwrap_or(repositories.product_root()),
            PathBuf::from(path.trim_start_matches('/')),
        ),
        None => (repositories.product_root(), reference.to_path_buf()),
    };
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(invalid_base_path(reference));
    }
    Ok((root, relative))
}

fn invalid_base_path(reference: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "base manifest must be a repository-relative path: {}",
            reference.display()
        ),
    )
}

#[cfg(test)]
#[path = "system_base_tests.rs"]
mod system_base_tests;
