//! Catalog discovery, package paths, and deterministic manifest walks.

use std::io;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::reference::PackageKey;

pub(crate) fn root(manifest: &Path) -> io::Result<PathBuf> {
    let canonical = manifest.canonicalize()?;
    canonical
        .ancestors()
        .skip(1)
        .find(|path| path.join("pkg").is_dir())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "cannot find catalog package directory above {}",
                    manifest.display()
                ),
            )
        })
}

pub(crate) fn package_manifest(catalog: &Path, key: &PackageKey) -> PathBuf {
    catalog
        .join("pkg")
        .join(&key.namespace)
        .join(format!("{}.yaml", key.slug))
}

pub(crate) fn visit_manifests(
    catalog: &Path,
    mut visit: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let root = catalog.join("pkg");
    for entry in WalkDir::new(&root).sort_by_file_name() {
        let entry = entry?;
        if !entry.file_type().is_file() || !is_yaml(entry.path()) {
            continue;
        }
        let relative = entry.path().strip_prefix(&root).map_err(io::Error::other)?;
        visit(entry.path(), relative)?;
    }
    Ok(())
}

fn is_yaml(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "yaml" || extension == "yml")
}
