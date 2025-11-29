use std::collections::{HashMap, HashSet};
use std::io;
use std::path::PathBuf;

use crate::manifest::ManifestIndex;
use crate::ostree_native::OstreeRepo;
use crate::utils::hash_file_content;

use super::types::{MaterializeRequest, RuntimeClosure};

/// Resolve runtime dependencies using precomputed deps from manifests.
///
/// This is the primary resolution method. It reads deps from:
/// - manifest FileEntry.needs (list of file paths)
/// - manifest resolution map (file_path -> dependency_name or @self)
///
/// No ELF scanning needed at install time.
pub fn resolve_runtime_deps_precomputed(
    repo_path: &str,
    requests: &[MaterializeRequest],
    manifest_index: &ManifestIndex,
) -> io::Result<RuntimeClosure> {
    let mut closure = RuntimeClosure::default();
    let mut visited: HashSet<String> = HashSet::new();
    let mut pending: Vec<String> = Vec::new();
    // cache: (manifest_ptr, dep_name) -> resolved_commit
    let mut dep_commit_cache: HashMap<(usize, String), String> = HashMap::new();

    let repo = OstreeRepo::open(repo_path)?;

    // seed with initial requests (mark as roots)
    for req in requests {
        let commit = req.commit().to_string();
        if !visited.contains(&commit) {
            closure.add_root(&commit, format!("requested: {}", commit));
            pending.push(commit.clone());
            visited.insert(commit);
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

        // extract output name from commit (e.g., ".../outputs/bin" -> "bin", ".../bundles/full" -> "full")
        let commit_parts: Vec<&str> = commit.split('/').collect();
        let commit_type = commit_parts
            .get(commit_parts.len().saturating_sub(2))
            .copied()
            .unwrap_or("");
        let commit_name = commit_parts.last().copied().unwrap_or("");

        // determine which outputs to process
        let output_names: Vec<String> = if commit_type == "bundles" {
            // bundle - expand to constituent outputs
            match manifest.bundles.get(commit_name) {
                Some(bundle) => bundle.includes.clone(),
                None => continue, // bundle not in manifest
            }
        } else {
            // single output
            vec![commit_name.to_string()]
        };

        // use manifest pointer as cache key component
        let manifest_key = manifest as *const _ as usize;

        // collect all dependencies needed by these outputs
        for output_name in &output_names {
            let output_spec = match manifest.outputs.get(output_name) {
                Some(spec) => spec,
                None => continue, // output not in manifest
            };

            for file_entry in &output_spec.files {
                for needed_file in &file_entry.needs {
                    // resolve file path to dependency name via resolution map
                    let dep_name = match manifest.resolution.get(needed_file) {
                        Some(dn) => dn.clone(),
                        None => {
                            closure.add_unresolved(
                                needed_file,
                                format!(
                                    "{} needs {} (no resolution)",
                                    file_entry.path, needed_file
                                ),
                            );
                            continue;
                        }
                    };

                    // skip @self entries - internal libs don't need resolution
                    if dep_name == "@self" {
                        continue;
                    }

                    // resolve dependency name to commit
                    let cache_key = (manifest_key, dep_name.clone());
                    let dep_commit = if let Some(cached) = dep_commit_cache.get(&cache_key) {
                        cached.clone()
                    } else {
                        // find dependency by name and derive commit
                        match resolve_dependency_to_commit(
                            &dep_name,
                            manifest,
                            manifest_index,
                            &repo,
                        ) {
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
                                        file_entry.path,
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
                                        file_entry.path, dep_name, pkg
                                    ),
                                );
                                continue;
                            }
                            ResolveResult::NotFound => {
                                closure.add_unresolved(
                                    &dep_name,
                                    format!(
                                        "{} needs dependency {} (not found)",
                                        file_entry.path, dep_name
                                    ),
                                );
                                continue;
                            }
                        }
                    };

                    let reason = format!(
                        "{} needs {} from {}",
                        file_entry.path, needed_file, dep_name
                    );

                    if !visited.contains(&dep_commit) {
                        // track the specific file needed from this commit
                        closure.add_file_dep(&dep_commit, needed_file, reason);
                        pending.push(dep_commit.clone());
                        visited.insert(dep_commit);
                    } else {
                        // commit already visited, but add this file too
                        closure
                            .files_needed
                            .entry(dep_commit.clone())
                            .or_default()
                            .insert(needed_file.clone());
                        closure
                            .reasons
                            .entry(dep_commit)
                            .or_default()
                            .insert(reason);
                    }
                }
            }
        }
    }

    Ok(closure)
}

/// Find the manifest that corresponds to an OSTree commit ref.
fn find_manifest_for_commit<'a>(
    commit: &str,
    index: &'a ManifestIndex,
) -> Option<&'a crate::manifest::types::Manifest> {
    // parse commit: x86_64/pkg/namespace/slug/version/outputs/name
    let parts: Vec<&str> = commit.split('/').collect();

    // find "pkg" position
    let pkg_idx = parts.iter().position(|&p| p == "pkg")?;

    // find "outputs" or "bundles" position
    let end_idx = parts
        .iter()
        .position(|&p| p == "outputs" || p == "bundles")?;

    if end_idx <= pkg_idx + 2 {
        return None;
    }

    // namespace is everything between pkg and version (version is second-to-last before outputs)
    // e.g., pkg/libs/system/glibc/2.39/outputs/lib -> namespace="libs/system", slug="glibc"
    let version_idx = end_idx - 1;
    let slug_idx = version_idx - 1;

    if slug_idx <= pkg_idx {
        return None;
    }

    let namespace = parts[pkg_idx + 1..slug_idx].join("/");
    let slug = parts[slug_idx];

    index.get_manifest(&namespace, slug)
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

/// Resolve a dependency name to an OSTree commit ref.
/// Uses {checksum}/files for stable checksums, {git_blob_sha}/files for bootstrap packages.
fn resolve_dependency_to_commit(
    dep_name: &str,
    source_manifest: &crate::manifest::types::Manifest,
    manifest_index: &ManifestIndex,
    repo: &OstreeRepo,
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

    let files_ref = format!("{}/files", address_hash);
    if repo.resolve_ref(&files_ref).is_ok() {
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
    // note: these tests require an actual OSTree repo, so they're integration tests
    // unit tests for the logic itself can be done with mocks
}
