//! Resolve runtime dependency closures from manifest metadata.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::io;
use std::path::PathBuf;

use crate::manifest::ManifestIndex;
use crate::store::Store;

use super::resolver_entries::{file_entries_to_process, is_checksum_files_commit_ref};
use super::resolver_metadata::missing_runtime_metadata;
use super::resolver_refs::{resolve_dependency_to_commit, ResolveResult};
use super::types::{MaterializeRequest, RuntimeClosure};

/// Resolve runtime dependencies using precomputed deps from manifests.
///
/// This is the primary resolution method. It reads deps from:
/// - manifest FileEntry.needs (list of file paths)
/// - manifest resolution map (file_path -> dependency_name or self)
///
/// No ELF scanning needed at install time.
pub fn resolve_runtime_deps_precomputed(
    repo_path: &str,
    requests: &[MaterializeRequest],
    manifest_index: &ManifestIndex,
    fallback_repos: &[PathBuf],
) -> io::Result<RuntimeClosure> {
    let store = Store::open_with_fallback_chain(repo_path, fallback_repos)?;
    let mut resolver = RuntimeResolver::new(manifest_index, store);
    resolver.seed_requests(requests);
    resolver.run();
    Ok(resolver.closure)
}

struct RuntimeResolver<'a> {
    manifest_index: &'a ManifestIndex,
    store: Store,
    closure: RuntimeClosure,
    visited: HashSet<String>,
    pending: Vec<String>,
    dep_commit_cache: HashMap<(usize, String), String>,
    processed_file_paths: HashMap<String, BTreeSet<String>>,
}

impl<'a> RuntimeResolver<'a> {
    fn new(manifest_index: &'a ManifestIndex, store: Store) -> Self {
        Self {
            manifest_index,
            store,
            closure: RuntimeClosure::default(),
            visited: HashSet::new(),
            pending: Vec::new(),
            dep_commit_cache: HashMap::new(),
            processed_file_paths: HashMap::new(),
        }
    }

    fn seed_requests(&mut self, requests: &[MaterializeRequest]) {
        for req in requests {
            let commit = req.commit().to_string();
            self.add_root_commit(&commit);
            if let MaterializeRequest::Files { paths, .. } = req {
                self.add_requested_files(&commit, paths);
            }
        }
    }

    fn add_root_commit(&mut self, commit: &str) {
        if self.visited.insert(commit.to_string()) {
            self.closure
                .add_root(commit, format!("requested: {}", commit));
            self.pending.push(commit.to_string());
        }
    }

    fn add_requested_files(&mut self, commit: &str, paths: &[String]) {
        let files = self
            .closure
            .files_needed
            .entry(commit.to_string())
            .or_default();
        files.extend(paths.iter().cloned());
    }

    fn run(&mut self) {
        while let Some(commit) = self.pending.pop() {
            self.process_commit(&commit);
        }
    }

    fn process_commit(&mut self, commit: &str) {
        let Some(manifest) = find_manifest_for_commit(commit, self.manifest_index) else {
            self.closure.add_unresolved(
                format!("manifest for {}", commit).as_str(),
                format!("{} was included in the runtime closure", commit),
            );
            return;
        };
        if self.record_missing_runtime_metadata(commit, manifest) {
            return;
        }
        let file_entries = file_entries_to_process(
            commit,
            manifest,
            &self.closure,
            &mut self.processed_file_paths,
        );
        self.record_missing_file_metadata(commit, &file_entries.missing_files);

        let manifest_key = manifest as *const _ as usize;
        for (file_path, file_needs) in file_entries.entries {
            self.process_file_needs(commit, manifest, manifest_key, &file_path, file_needs);
        }
    }

    fn process_file_needs(
        &mut self,
        commit: &str,
        manifest: &crate::manifest::types::Manifest,
        manifest_key: usize,
        file_path: &str,
        file_needs: Vec<String>,
    ) {
        for needed_file in file_needs {
            self.process_needed_file(commit, manifest, manifest_key, file_path, &needed_file);
        }
    }

    fn record_missing_runtime_metadata(
        &mut self,
        commit: &str,
        manifest: &crate::manifest::types::Manifest,
    ) -> bool {
        let missing = missing_runtime_metadata(commit, manifest);
        for item in &missing {
            self.closure.add_unresolved(
                item,
                format!("{} references missing runtime metadata", commit),
            );
        }
        !missing.is_empty()
    }

    fn record_missing_file_metadata(&mut self, commit: &str, missing_files: &[String]) {
        for file_path in missing_files {
            self.closure.add_unresolved(
                file_path,
                format!(
                    "{} requested {} but no manifest file entry describes it",
                    commit, file_path
                ),
            );
        }
    }

    fn process_needed_file(
        &mut self,
        commit: &str,
        manifest: &crate::manifest::types::Manifest,
        manifest_key: usize,
        file_path: &str,
        needed_file: &str,
    ) {
        let Some(dep_name) = self.dependency_name(manifest, file_path, needed_file) else {
            return;
        };
        if dep_name == "self" {
            queue_self_file_dependency(
                commit,
                needed_file,
                format!("{} needs {} from self", file_path, needed_file),
                &mut self.closure,
                &mut self.pending,
            );
            return;
        }

        if let Some(dep_commit) =
            self.dependency_commit(manifest, manifest_key, file_path, &dep_name)
        {
            self.queue_dependency_file(dep_commit, needed_file, file_path, &dep_name);
        }
    }

    fn dependency_name(
        &mut self,
        manifest: &crate::manifest::types::Manifest,
        file_path: &str,
        needed_file: &str,
    ) -> Option<String> {
        match manifest.resolution.get(needed_file) {
            Some(dep_name) => Some(dep_name.clone()),
            None => {
                self.closure.add_unresolved(
                    needed_file,
                    format!("{} needs {} (no resolution)", file_path, needed_file),
                );
                None
            }
        }
    }

    fn dependency_commit(
        &mut self,
        manifest: &crate::manifest::types::Manifest,
        manifest_key: usize,
        file_path: &str,
        dep_name: &str,
    ) -> Option<String> {
        let cache_key = (manifest_key, dep_name.to_string());
        if let Some(cached) = self.dep_commit_cache.get(&cache_key) {
            return Some(cached.clone());
        }

        match resolve_dependency_to_commit(dep_name, manifest, self.manifest_index, &self.store) {
            ResolveResult::Ok(commit) => {
                self.dep_commit_cache.insert(cache_key, commit.clone());
                Some(commit)
            }
            result => {
                self.record_resolve_failure(result, file_path, dep_name);
                None
            }
        }
    }

    fn record_resolve_failure(&mut self, result: ResolveResult, file_path: &str, dep_name: &str) {
        match result {
            ResolveResult::FilesCommitMissing { package, checksum } => {
                self.closure.add_unresolved(
                    &format!("{} (run: nex compute-deps pkg/{}.yaml)", package, package),
                    format!(
                        "{} needs {} - missing {}/files commit",
                        file_path,
                        dep_name,
                        &checksum[..12]
                    ),
                );
            }
            ResolveResult::ManifestNotFound(package) => {
                self.closure.add_unresolved(
                    dep_name,
                    format!(
                        "{} needs {} - manifest not found for {}",
                        file_path, dep_name, package
                    ),
                );
            }
            ResolveResult::NotFound => {
                self.closure.add_unresolved(
                    dep_name,
                    format!("{} needs dependency {} (not found)", file_path, dep_name),
                );
            }
            ResolveResult::Ok(_) => {}
        }
    }

    fn queue_dependency_file(
        &mut self,
        dep_commit: String,
        needed_file: &str,
        file_path: &str,
        dep_name: &str,
    ) {
        let reason = format!("{} needs {} from {}", file_path, needed_file, dep_name);
        let added_file = self.closure.add_file_dep(&dep_commit, needed_file, reason);
        if self.visited.insert(dep_commit.clone())
            || added_file && is_checksum_files_commit_ref(&dep_commit)
        {
            self.pending.push(dep_commit);
        }
    }
}

fn queue_self_file_dependency(
    commit: &str,
    needed_file: &str,
    reason: String,
    closure: &mut RuntimeClosure,
    pending: &mut Vec<String>,
) {
    if !is_checksum_files_commit_ref(commit) {
        return;
    }

    if closure.add_file_dep(commit, needed_file, reason) {
        pending.push(commit.to_string());
    }
}

/// Find the manifest that corresponds to a store commit ref.
fn find_manifest_for_commit<'a>(
    commit: &str,
    index: &'a ManifestIndex,
) -> Option<&'a crate::manifest::types::Manifest> {
    use crate::refs::PackageRef;

    if let Ok(pkg_ref) = PackageRef::parse(commit) {
        if let Some(manifest) = index.get_manifest(&pkg_ref.namespace, &pkg_ref.slug) {
            return Some(manifest);
        }
    }

    let (namespace, slug) = checksum_files_identity(commit)?;
    index.get_manifest(&namespace, &slug)
}

fn checksum_files_identity(commit: &str) -> Option<(String, String)> {
    if !is_checksum_files_commit_ref(commit) {
        return None;
    }

    let parts: Vec<&str> = commit.split('/').collect();
    if parts.get(1) != Some(&"pkg") {
        return None;
    }

    let address_idx = parts.len().checked_sub(2)?;
    let slug_idx = address_idx.checked_sub(2)?;
    if slug_idx <= 2 {
        return None;
    }

    Some((parts[2..slug_idx].join("/"), parts[slug_idx].to_string()))
}

#[cfg(test)]
#[path = "resolver_tests.rs"]
mod resolver_tests;
