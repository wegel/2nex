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

    /// Report whether a manifest belongs to the writable product repository.
    pub fn is_writable(&self, manifest_path: &Path) -> io::Result<bool> {
        Ok(repository_root_for_path(manifest_path)? == self.product_root)
    }
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
