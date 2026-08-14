use serde_yaml::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::inheritance;
use super::repositories::repository_root_for_path;
use super::types::*;

/// Load manifest from a ManifestSource (path or blob)
/// For system manifests loaded from path, inheritance is resolved
pub fn load_manifest_from_source(source: &ManifestSource) -> io::Result<ManifestData> {
    match source {
        ManifestSource::Path(path) => {
            // check if it's a system manifest to resolve inheritance
            let content = fs::read_to_string(path)?;
            let doc: Value = serde_yaml::from_str(&content)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            if detect_manifest_kind(&doc) == ManifestKind::System {
                let resolved = load_system_manifest_resolved(path)?;
                Ok(ManifestData::System(resolved))
            } else {
                let root = manifest_repository_root(path);
                load_manifest_from_str_in_repository(&content, &root)
            }
        }
        ManifestSource::Blob {
            sha,
            path,
            git_root,
        } => {
            let content = crate::utils::fetch_git_blob(&git_root, sha).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("failed to fetch blob {} for {}: {}", sha, path.display(), e),
                )
            })?;
            // note: blob manifests don't support inheritance
            load_manifest_from_str_in_repository(&content, git_root)
        }
        ManifestSource::Skip => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot load manifest from Skip marker",
        )),
    }
}

/// Compute manifest hash from a ManifestSource
pub fn compute_manifest_hash_from_source(source: &ManifestSource) -> io::Result<String> {
    use sha2::{Digest, Sha256};

    let content = match source {
        ManifestSource::Path(path) => fs::read(path)?,
        ManifestSource::Blob { sha, git_root, .. } => {
            crate::utils::fetch_git_blob(&git_root, sha)?.into_bytes()
        }
        ManifestSource::Skip => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot compute hash for Skip marker",
            ))
        }
    };

    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn detect_manifest_kind(doc: &Value) -> ManifestKind {
    if let Some(kind) = doc.get("kind").and_then(|v| v.as_str()) {
        if kind.eq_ignore_ascii_case("system") {
            return ManifestKind::System;
        }
    }
    if doc.get("system").is_some() && doc.get("package").is_none() {
        ManifestKind::System
    } else {
        ManifestKind::Package
    }
}

pub fn validate_system_manifest(manifest: &SystemManifest) -> io::Result<()> {
    if manifest.packages.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "System manifests must specify at least one entry under 'packages'",
        ));
    }
    if manifest.system.version.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "System manifests must set system.version",
        ));
    }
    Ok(())
}

pub fn load_manifest(file_path: &str) -> io::Result<ManifestData> {
    let manifest_str = fs::read_to_string(file_path)?;
    let root = manifest_repository_root(Path::new(file_path));
    load_manifest_from_str_in_repository(&manifest_str, &root)
}

/// Load manifest from string content (for blob-ref mode)
pub fn load_manifest_from_str(manifest_str: &str) -> io::Result<ManifestData> {
    let doc: Value = serde_yaml::from_str(manifest_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    match detect_manifest_kind(&doc) {
        ManifestKind::Package => {
            let manifest: Manifest = serde_yaml::from_value(doc)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Ok(ManifestData::Package(manifest))
        }
        ManifestKind::System => {
            let sys: SystemManifest = serde_yaml::from_value(doc)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            validate_system_manifest(&sys)?;
            Ok(ManifestData::System(sys))
        }
    }
}

pub(super) fn resolve_system_paths(manifest: &mut SystemManifest, repository_root: &Path) {
    resolve_sources(&mut manifest.sources, repository_root);
    for overlay in &mut manifest.overlays {
        if overlay.is_relative() {
            *overlay = repository_root.join(&*overlay);
        }
    }
}

fn load_manifest_from_str_in_repository(
    manifest_str: &str,
    repository_root: &Path,
) -> io::Result<ManifestData> {
    let mut manifest = load_manifest_from_str(manifest_str)?;
    match &mut manifest {
        ManifestData::Package(package) => resolve_sources(&mut package.sources, repository_root),
        ManifestData::System(system) => resolve_system_paths(system, repository_root),
    }
    Ok(manifest)
}

fn resolve_sources(sources: &mut [Source], repository_root: &Path) {
    for source in sources {
        resolve_local_reference(&mut source.file, repository_root);
        resolve_local_reference(&mut source.dev, repository_root);
        resolve_local_reference(&mut source.cargo_lock, repository_root);
        resolve_local_reference(&mut source.cargo_toml, repository_root);
        resolve_local_reference(&mut source.go_sum, repository_root);
        resolve_local_reference(&mut source.zig_zon, repository_root);
    }
}

fn resolve_local_reference(reference: &mut Option<String>, repository_root: &Path) {
    let Some(value) = reference else {
        return;
    };
    if is_remote_reference(value) || Path::new(value.as_str()).is_absolute() {
        return;
    }
    *value = repository_root
        .join(value.as_str())
        .to_string_lossy()
        .into_owned();
}

fn is_remote_reference(reference: &str) -> bool {
    reference.starts_with("http://") || reference.starts_with("https://")
}

fn manifest_repository_root(manifest_path: &Path) -> PathBuf {
    repository_root_for_path(manifest_path)
        .or_else(|_| std::env::current_dir())
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Load a system manifest with inheritance resolution
pub fn load_system_manifest_resolved(file_path: &Path) -> io::Result<SystemManifest> {
    let resolved = inheritance::resolve_inheritance(file_path)?;
    validate_system_manifest(&resolved)?;
    Ok(resolved)
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod parser_tests;
