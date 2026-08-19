//! Conventional product and upstream Nex manifest repository discovery.

use std::io;
use std::path::{Path, PathBuf};

const UPSTREAM_NEX: &str = "upstream/nex";

/// Manifest repositories available to one product build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestRepositories {
    product_root: PathBuf,
    upstream_root: Option<PathBuf>,
}

impl ManifestRepositories {
    /// Discover the product repository from a requested manifest or directory.
    pub fn discover(path: &Path) -> io::Result<Self> {
        let product_root = repository_root_for_path(path)?;
        let upstream_root = discover_upstream_root(&product_root)?;
        Ok(Self {
            product_root,
            upstream_root,
        })
    }

    /// Return the product repository, which owns writable product manifests.
    pub fn product_root(&self) -> &Path {
        &self.product_root
    }

    /// Return package manifest directories in the conventional search set.
    pub fn package_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        push_package_dir(&mut dirs, &self.product_root);
        if let Some(root) = &self.upstream_root {
            push_package_dir(&mut dirs, root);
        }
        dirs
    }

    /// Return the upstream Nex repository when one is separate from the product.
    pub fn upstream_root(&self) -> Option<&Path> {
        self.upstream_root.as_deref()
    }

    /// Report whether a manifest belongs to the writable product repository.
    pub fn is_writable(&self, manifest_path: &Path) -> io::Result<bool> {
        Ok(repository_root_for_path(manifest_path)? == self.product_root)
    }
}

/// Prefix that names the upstream Nex repository in a manifest reference.
pub const NEX_REPOSITORY_PREFIX: &str = "nex:";

/// Resolve a manifest reference that may name a repository.
///
/// A plain relative path resolves against `owning_root`, which keeps every
/// existing manifest working. A reference that starts with `nex:` resolves
/// against the upstream Nex repository, so a product manifest does not encode
/// how deeply it sits below or above that repository. When no separate
/// upstream repository exists, the owning repository is itself Nex.
pub fn resolve_repository_reference(
    reference: &Path,
    owning_root: &Path,
    upstream_root: Option<&Path>,
) -> PathBuf {
    let text = reference.to_string_lossy();
    if let Some(rest) = text.strip_prefix(NEX_REPOSITORY_PREFIX) {
        let rest = rest.trim_start_matches('/');
        return upstream_root.unwrap_or(owning_root).join(rest);
    }
    if reference.is_absolute() {
        return reference.to_path_buf();
    }
    owning_root.join(reference)
}

/// Find the nearest Git repository that owns an existing path.
pub fn repository_root_for_path(path: &Path) -> io::Result<PathBuf> {
    let canonical = path.canonicalize().map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to resolve {}: {}", path.display(), error),
        )
    })?;
    let start = if canonical.is_dir() {
        canonical.as_path()
    } else {
        canonical.parent().unwrap_or(canonical.as_path())
    };

    start
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{} is not inside a Git repository", path.display()),
            )
        })
}

fn discover_upstream_root(product_root: &Path) -> io::Result<Option<PathBuf>> {
    let candidate = product_root.join(UPSTREAM_NEX);
    if !candidate.join(".git").exists() || !candidate.join("pkg").is_dir() {
        return Ok(None);
    }
    candidate.canonicalize().map(Some).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to resolve {}: {}", candidate.display(), error),
        )
    })
}

fn push_package_dir(dirs: &mut Vec<PathBuf>, repository_root: &Path) {
    let package_dir = repository_root.join("pkg");
    if package_dir.is_dir() {
        dirs.push(package_dir);
    }
}

#[cfg(test)]
#[path = "repositories_tests.rs"]
mod repositories_tests;
