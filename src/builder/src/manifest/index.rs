//! ManifestIndex: loads manifests from a directory and indexes file providers.
//!
//! This is the core of the Manifest-as-Database pattern for JIT resolution.
//! Instead of scanning OSTree refs, we read the manifest files which declare
//! exactly what files each package provides.

use std::collections::HashMap;
use std::io;
use std::path::Path;

use walkdir::WalkDir;

use super::parser::load_manifest_from_source;
use super::types::{Manifest, ManifestData, ManifestSource};

/// Index of manifests for provider resolution.
///
/// Maps library basenames (e.g., "libc.so.6") and full paths to the OSTree refs
/// that provide them, based on manifest `outputs` sections.
#[derive(Debug, Default)]
pub struct ManifestIndex {
    /// map from basename (e.g., "libc.so.6") to list of (commit_ref, full_path)
    by_basename: HashMap<String, Vec<(String, String)>>,
    /// map from full path (e.g., "/usr/lib/libc.so.6") to list of commit_refs
    by_path: HashMap<String, Vec<String>>,
    /// loaded manifests by their namespace/slug (e.g., "libs/system/glibc")
    manifests: HashMap<String, Manifest>,
}

impl ManifestIndex {
    /// Create an empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load all manifests from a directory (e.g., /nex/db/pkg or pkg/).
    ///
    /// Recursively walks the directory and parses all .yaml/.yml files as package manifests.
    pub fn load<P: AsRef<Path>>(pkg_dir: P) -> io::Result<Self> {
        let pkg_dir = pkg_dir.as_ref();
        if !pkg_dir.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Manifest directory not found: {}", pkg_dir.display()),
            ));
        }

        let mut index = Self::new();

        for entry in WalkDir::new(pkg_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str());
            if ext != Some("yaml") && ext != Some("yml") {
                continue;
            }

            // try to parse as package manifest
            match load_manifest_from_source(&ManifestSource::Path(path.to_path_buf())) {
                Ok(ManifestData::Package(manifest)) => {
                    index.add_manifest(manifest);
                }
                Ok(ManifestData::System(_)) | Err(_) => {
                    // skip system manifests and files that aren't valid manifests
                    continue;
                }
            }
        }

        Ok(index)
    }

    /// Add a manifest to the index, indexing its outputs.
    pub fn add_manifest(&mut self, manifest: Manifest) {
        let key = format!("{}/{}", manifest.package.namespace, manifest.package.slug);
        let arch = "x86_64"; // TODO: make configurable

        // index each output's files
        for (output_name, output_spec) in &manifest.outputs {
            // construct the OSTree ref for this output
            let commit_ref = format!(
                "{}/{}/{}/{}/outputs/{}",
                arch,
                manifest.package.namespace_path(),
                manifest.package.slug,
                manifest.package.version,
                output_name
            );

            for file_entry in &output_spec.files {
                let file_path = &file_entry.path;
                // index by full path (multiple providers possible)
                self.by_path
                    .entry(file_path.clone())
                    .or_default()
                    .push(commit_ref.clone());

                // index by basename for library lookups
                if let Some(basename) = Path::new(file_path).file_name().and_then(|n| n.to_str()) {
                    self.by_basename
                        .entry(basename.to_string())
                        .or_default()
                        .push((commit_ref.clone(), file_path.clone()));
                }
            }
        }

        self.manifests.insert(key, manifest);
    }

    /// Resolve a library name to its provider commit ref.
    ///
    /// Accepts either a basename (e.g., "libc.so.6") or full path ("/usr/lib/libc.so.6").
    /// Returns the OSTree commit ref that provides the file.
    /// When multiple providers exist, prefers non-bootstrap packages over bootstrap.
    pub fn resolve(&self, name: &str) -> Option<&str> {
        // try full path first - but apply priority selection
        if let Some(providers) = self.by_path.get(name) {
            return Self::select_best_from_commits(providers);
        }

        // try basename - select best provider by priority
        if let Some(providers) = self.by_basename.get(name) {
            return Self::select_best_provider(providers);
        }

        // try extracting basename from path
        if let Some(basename) = Path::new(name).file_name().and_then(|n| n.to_str()) {
            if let Some(providers) = self.by_basename.get(basename) {
                return Self::select_best_provider(providers);
            }
        }

        None
    }

    /// Select best provider from a list of commits based on phase priority.
    fn select_best_from_commits(commits: &[String]) -> Option<&str> {
        if commits.is_empty() {
            return None;
        }
        if commits.len() == 1 {
            return Some(&commits[0]);
        }

        commits
            .iter()
            .max_by_key(|commit| Self::phase_priority(commit))
            .map(|s| s.as_str())
    }

    /// Select best provider from a list based on phase priority.
    /// Non-bootstrap packages have priority over bootstrap packages.
    fn select_best_provider(providers: &[(String, String)]) -> Option<&str> {
        if providers.is_empty() {
            return None;
        }
        if providers.len() == 1 {
            return Some(&providers[0].0);
        }

        // find highest priority provider
        providers
            .iter()
            .max_by_key(|(commit, _)| Self::phase_priority(commit))
            .map(|(commit, _)| commit.as_str())
    }

    /// Get phase priority from commit path (higher = later phase = higher priority).
    fn phase_priority(commit: &str) -> u32 {
        if commit.contains("/bootstrap/phase0/") {
            0
        } else if commit.contains("/bootstrap/phase1/") {
            1
        } else if commit.contains("/bootstrap/phase2/") {
            2
        } else if commit.contains("/bootstrap/phase3/") {
            3
        } else {
            // non-bootstrap packages have highest priority
            100
        }
    }

    /// Get all providers for a library name.
    ///
    /// Returns a list of (commit_ref, full_path) pairs.
    pub fn resolve_all(&self, name: &str) -> Vec<(&str, &str)> {
        let basename = Path::new(name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(name);

        self.by_basename
            .get(basename)
            .map(|v| v.iter().map(|(c, p)| (c.as_str(), p.as_str())).collect())
            .unwrap_or_default()
    }

    /// Get a manifest by namespace/slug.
    pub fn get_manifest(&self, namespace: &str, slug: &str) -> Option<&Manifest> {
        let key = format!("{}/{}", namespace, slug);
        self.manifests.get(&key)
    }

    /// Get all loaded manifests.
    pub fn manifests(&self) -> impl Iterator<Item = &Manifest> {
        self.manifests.values()
    }

    /// Number of manifests loaded.
    pub fn manifest_count(&self) -> usize {
        self.manifests.len()
    }

    /// Number of indexed files.
    pub fn file_count(&self) -> usize {
        self.by_path.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_index() {
        let index = ManifestIndex::new();
        assert_eq!(index.manifest_count(), 0);
        assert_eq!(index.file_count(), 0);
        assert!(index.resolve("libc.so.6").is_none());
    }
}
