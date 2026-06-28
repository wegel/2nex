use std::collections::{BTreeSet, HashMap, HashSet};
use std::io;
use std::path::PathBuf;

use crate::manifest::ManifestIndex;
use crate::store::Store;
use crate::utils::hash_file_content;

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
    let mut closure = RuntimeClosure::default();
    let mut visited: HashSet<String> = HashSet::new();
    let mut pending: Vec<String> = Vec::new();
    // cache: (manifest_ptr, dep_name) -> resolved_commit
    let mut dep_commit_cache: HashMap<(usize, String), String> = HashMap::new();
    let mut processed_file_paths: HashMap<String, BTreeSet<String>> = HashMap::new();

    let store = Store::open_with_fallback_chain(repo_path, fallback_repos)?;

    // seed with initial requests (mark as roots)
    for req in requests {
        let commit = req.commit().to_string();
        if !visited.contains(&commit) {
            closure.add_root(&commit, format!("requested: {}", commit));
            pending.push(commit.clone());
            visited.insert(commit.clone());
        }
        if let MaterializeRequest::Files { paths, .. } = req {
            for path in paths {
                closure
                    .files_needed
                    .entry(commit.clone())
                    .or_default()
                    .insert(path.clone());
            }
        }
    }

    // transitive resolution loop
    while let Some(commit) = pending.pop() {
        // find the manifest for this commit
        let manifest = match find_manifest_for_commit(&commit, manifest_index) {
            Some(m) => m,
            None => {
                // no manifest found - can't resolve deps for this commit
                // this is expected for commits that haven't been processed by compute-deps
                continue;
            }
        };

        let file_entries =
            file_entries_to_process(&commit, manifest, &closure, &mut processed_file_paths);
        if file_entries.is_empty() {
            continue;
        }

        // use manifest pointer as cache key component
        let manifest_key = manifest as *const _ as usize;

        // collect all dependencies needed by these files
        for (file_path, file_needs) in file_entries {
            for needed_file in file_needs {
                // resolve file path to dependency name via resolution map
                let dep_name = match manifest.resolution.get(&needed_file) {
                    Some(dn) => dn.clone(),
                    None => {
                        closure.add_unresolved(
                            &needed_file,
                            format!("{} needs {} (no resolution)", file_path, needed_file),
                        );
                        continue;
                    }
                };

                if dep_name == "self" {
                    queue_self_file_dependency(
                        &commit,
                        &needed_file,
                        format!("{} needs {} from self", file_path, needed_file),
                        &mut closure,
                        &mut pending,
                    );
                    continue;
                }

                // resolve dependency name to commit
                let cache_key = (manifest_key, dep_name.clone());
                let dep_commit = if let Some(cached) = dep_commit_cache.get(&cache_key) {
                    cached.clone()
                } else {
                    // find dependency by name and derive commit
                    match resolve_dependency_to_commit(&dep_name, manifest, manifest_index, &store)
                    {
                        ResolveResult::Ok(c) => {
                            dep_commit_cache.insert(cache_key, c.clone());
                            c
                        }
                        ResolveResult::FilesCommitMissing { package, checksum } => {
                            closure.add_unresolved(
                                &format!(
                                    "{} (run: nex compute-deps pkg/{}.yaml)",
                                    package, package
                                ),
                                format!(
                                    "{} needs {} - missing {}/files commit",
                                    file_path,
                                    dep_name,
                                    &checksum[..12]
                                ),
                            );
                            continue;
                        }
                        ResolveResult::ManifestNotFound(pkg) => {
                            closure.add_unresolved(
                                &dep_name,
                                format!(
                                    "{} needs {} - manifest not found for {}",
                                    file_path, dep_name, pkg
                                ),
                            );
                            continue;
                        }
                        ResolveResult::NotFound => {
                            closure.add_unresolved(
                                &dep_name,
                                format!("{} needs dependency {} (not found)", file_path, dep_name),
                            );
                            continue;
                        }
                    }
                };

                let reason = format!("{} needs {} from {}", file_path, needed_file, dep_name);

                if !visited.contains(&dep_commit) {
                    // track the specific file needed from this commit
                    closure.add_file_dep(&dep_commit, &needed_file, reason);
                    pending.push(dep_commit.clone());
                    visited.insert(dep_commit);
                } else {
                    let added_file = closure.add_file_dep(&dep_commit, &needed_file, reason);
                    if added_file && is_checksum_files_commit_ref(&dep_commit) {
                        pending.push(dep_commit);
                    }
                }
            }
        }
    }

    Ok(closure)
}

fn file_entries_to_process(
    commit: &str,
    manifest: &crate::manifest::types::Manifest,
    closure: &RuntimeClosure,
    processed_file_paths: &mut HashMap<String, BTreeSet<String>>,
) -> Vec<(String, Vec<String>)> {
    if is_checksum_files_commit_ref(commit) {
        let requested_files = match closure.get_files(commit) {
            Some(files) => files,
            None => return Vec::new(),
        };
        let processed_files = processed_file_paths.entry(commit.to_string()).or_default();
        let pending_files: BTreeSet<String> = requested_files
            .difference(processed_files)
            .cloned()
            .collect();
        if pending_files.is_empty() {
            return Vec::new();
        }

        processed_files.extend(pending_files.iter().cloned());
        return manifest
            .outputs
            .values()
            .flat_map(|output| output.files.iter())
            .filter(|file_entry| pending_files.contains(&file_entry.path))
            .map(|file_entry| (file_entry.path.clone(), file_entry.needs.clone()))
            .collect();
    }

    let commit_parts: Vec<&str> = commit.split('/').collect();
    let commit_type = commit_parts
        .get(commit_parts.len().saturating_sub(2))
        .copied()
        .unwrap_or("");
    let commit_name = commit_parts.last().copied().unwrap_or("");

    let output_names: Vec<String> = if commit_type == "bundles" {
        match manifest.bundles.get(commit_name) {
            Some(bundle) => bundle.includes.clone(),
            None => return Vec::new(),
        }
    } else {
        vec![commit_name.to_string()]
    };

    output_names
        .iter()
        .filter_map(|output_name| manifest.outputs.get(output_name))
        .flat_map(|output| output.files.iter())
        .map(|file_entry| (file_entry.path.clone(), file_entry.needs.clone()))
        .collect()
}

fn is_checksum_files_commit_ref(commit: &str) -> bool {
    let mut parts = commit.rsplit('/');
    let Some(last) = parts.next() else {
        return false;
    };
    let Some(previous) = parts.next() else {
        return false;
    };
    last == "files"
        && previous != "outputs"
        && previous != "bundles"
        && commit.split('/').count() >= 7
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

/// Result of resolving a dependency to a commit.
enum ResolveResult {
    /// Successfully resolved to a files commit
    Ok(String),
    /// Dependency not found in manifest
    NotFound,
    /// Manifest not found for dependency
    ManifestNotFound(String),
    /// Files commit doesn't exist - need to run compute-deps
    FilesCommitMissing { package: String, checksum: String },
}

/// Resolve a dependency name to a commit ref.
/// Uses x86_64/pkg/{ns}/{slug}/{ver}/{checksum}/files for stable checksums,
/// or {git_blob_sha}/files for bootstrap packages.
fn resolve_dependency_to_commit(
    dep_name: &str,
    source_manifest: &crate::manifest::types::Manifest,
    manifest_index: &ManifestIndex,
    store: &Store,
) -> ResolveResult {
    // find the dependency by name in the source manifest
    let dep = match source_manifest
        .dependencies
        .iter()
        .find(|d| d.name.as_deref() == Some(dep_name))
    {
        Some(d) => d,
        None => return ResolveResult::NotFound,
    };

    // parse the dependency's commit to extract package info
    // e.g., "x86_64/pkg/libs/system/glibc/2.39/outputs/lib"
    let parts: Vec<&str> = dep.commit.split('/').collect();
    let pkg_idx = match parts.iter().position(|&p| p == "pkg") {
        Some(i) => i,
        None => return ResolveResult::NotFound,
    };
    let end_idx = match parts.iter().position(|&p| p == "outputs" || p == "bundles") {
        Some(i) => i,
        None => return ResolveResult::NotFound,
    };

    if end_idx <= pkg_idx + 2 {
        return ResolveResult::NotFound;
    }

    let slug = parts[end_idx - 2];
    let version = parts[end_idx - 1];
    let namespace_path = parts[pkg_idx + 1..end_idx - 2].join("/");
    let package_path = format!("{}/{}", namespace_path, slug);

    // look up the dependency's manifest
    let dep_manifest = match manifest_index.get_manifest(&namespace_path, slug) {
        Some(m) => m,
        None => return ResolveResult::ManifestNotFound(package_path),
    };

    // determine address hash: use checksum if stable, else use manifest git blob SHA
    let has_stable_checksum = dep_manifest.package.checksum.is_some()
        && dep_manifest.package.stable_checksum.unwrap_or(true);

    let address_hash = if has_stable_checksum {
        dep_manifest.package.checksum.clone().unwrap()
    } else {
        // use manifest's git blob SHA (input-addressed for bootstrap packages)
        let manifest_path = PathBuf::from(format!("pkg/{}/{}.yaml", namespace_path, slug));
        match hash_file_content(&manifest_path) {
            Ok(h) => h,
            Err(_) => return ResolveResult::NotFound,
        }
    };

    let files_ref = format!(
        "x86_64/pkg/{}/{}/{}/{}/files",
        namespace_path, slug, version, address_hash
    );
    if store.resolve_ref(&files_ref).is_ok() {
        return ResolveResult::Ok(files_ref);
    }

    // files commit doesn't exist - need to run compute-deps
    ResolveResult::FilesCommitMissing {
        package: package_path,
        checksum: address_hash,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use crate::manifest::types::{Build, FileEntry, Manifest, OutputSpec, Package};

    use super::{file_entries_to_process, queue_self_file_dependency, RuntimeClosure};

    fn manifest_with_lib_output() -> Manifest {
        let mut outputs = HashMap::new();
        outputs.insert(
            "lib".to_string(),
            OutputSpec {
                files: vec![
                    FileEntry {
                        path: "/usr/lib/libX11.so.6".to_string(),
                        needs: vec!["/usr/lib/libxcb.so.1".to_string()],
                    },
                    FileEntry {
                        path: "/usr/lib/libX11-xcb.so.1".to_string(),
                        needs: Vec::new(),
                    },
                ],
            },
        );

        Manifest {
            package: Package {
                name: "libX11".to_string(),
                slug: "libx11".to_string(),
                version: "1.8.10".to_string(),
                namespace: "libs/x11".to_string(),
                checksum: Some("abc".to_string()),
                stable_checksum: None,
                seed: false,
            },
            dependencies: Vec::new(),
            sources: Vec::new(),
            build: Build {
                environment: String::new(),
                script: String::new(),
                profile: Vec::new(),
            },
            outputs,
            bundles: HashMap::new(),
            resolution: HashMap::new(),
        }
    }

    #[test]
    fn files_commit_processes_needed_manifest_entries_once() {
        let manifest = manifest_with_lib_output();
        let commit = "x86_64/pkg/libs/x11/libx11/1.8.10/abc/files";
        let mut closure = RuntimeClosure::default();
        closure.add_file_dep(commit, "/usr/lib/libX11.so.6", "test".to_string());

        let mut processed_file_paths = HashMap::new();
        let entries =
            file_entries_to_process(commit, &manifest, &closure, &mut processed_file_paths);
        assert_eq!(
            entries,
            vec![(
                "/usr/lib/libX11.so.6".to_string(),
                vec!["/usr/lib/libxcb.so.1".to_string()]
            )]
        );

        let entries =
            file_entries_to_process(commit, &manifest, &closure, &mut processed_file_paths);
        assert!(entries.is_empty());

        closure.add_file_dep(
            commit,
            "/usr/lib/libX11-xcb.so.1",
            "another need".to_string(),
        );
        let entries =
            file_entries_to_process(commit, &manifest, &closure, &mut processed_file_paths);
        assert_eq!(
            entries,
            vec![("/usr/lib/libX11-xcb.so.1".to_string(), Vec::new())]
        );
    }

    #[test]
    fn checksum_files_commit_queues_self_file_for_later_processing() {
        let commit = "x86_64/pkg/libs/graphics/mesa/24.2.7/abc/files";
        let mut closure = RuntimeClosure::default();
        let mut pending = Vec::new();

        queue_self_file_dependency(
            commit,
            "/usr/lib/libgallium-24.2.7.so",
            "test".to_string(),
            &mut closure,
            &mut pending,
        );

        assert_eq!(pending, vec![commit.to_string()]);
        assert_eq!(
            closure.get_files(commit).cloned().unwrap_or_default(),
            BTreeSet::from(["/usr/lib/libgallium-24.2.7.so".to_string()])
        );

        queue_self_file_dependency(
            commit,
            "/usr/lib/libgallium-24.2.7.so",
            "duplicate".to_string(),
            &mut closure,
            &mut pending,
        );

        assert_eq!(pending, vec![commit.to_string()]);
    }
}
