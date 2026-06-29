//! Dependency flattening for Nex capsules using precomputed deps.
//!
//! This module handles flattening runtime dependencies into each package's
//! `lib/` directory, creating self-contained "capsules" that nex-ld-shim can use.
//!
//! Dependencies are resolved transitively: if binary A needs libB.so, and libB.so
//! needs libC.so, we flatten both libB.so and libC.so into A's capsule.
//!
//! The resolution map points file paths to dependency names (or self for internal).
//! The files commit is derived from the dependency's manifest.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use crate::manifest::ManifestIndex;
use crate::utils::hash_file_content;

/// Flatten runtime dependencies for a single package capsule using precomputed deps.
///
/// Reads deps from the manifest and copies required libraries into the package's
/// `lib/` directory. Dependencies are resolved transitively - if libA needs libB,
/// we also flatten libB and everything it needs.
pub fn flatten_capsule_precomputed(
    repo_path: &str,
    pkg_dir: &Path,
    commit: &str,
    manifest_index: &ManifestIndex,
    fallback_repos: &[PathBuf],
) -> io::Result<usize> {
    // find manifest for this commit
    let manifest = match find_manifest_for_commit(commit, manifest_index) {
        Some(m) => m,
        None => {
            // no manifest - can't flatten without precomputed deps
            return Ok(0);
        }
    };

    // get the current package's files commit for self libs
    let self_files_commit = derive_files_commit_for_manifest(manifest);

    // extract output type and name from commit
    let commit_parts: Vec<&str> = commit.split('/').collect();
    let commit_type = commit_parts
        .get(commit_parts.len().saturating_sub(2))
        .copied()
        .unwrap_or("");
    let commit_name = commit_parts.last().copied().unwrap_or("");

    // determine which outputs to process
    // important: capsules may have multiple outputs merged (e.g., bin + lib),
    // so we need to process ALL outputs that have files present in the capsule
    let output_names: Vec<String> = if commit_type == "bundles" {
        // bundle - expand to constituent outputs
        match manifest.bundles.get(commit_name) {
            Some(bundle) => bundle.includes.clone(),
            None => return Ok(0),
        }
    } else {
        // single output specified, but check for other outputs in the capsule
        // this handles the case where system assembly merges multiple outputs
        detect_outputs_in_capsule(pkg_dir, manifest)
    };

    // collect all needed libs (both external deps and self libs)
    // for self libs, we flatten from own {checksum}/files commit
    // for external deps, we resolve transitively
    let mut self_libs: Vec<String> = Vec::new(); // file paths for self libs
    let mut external_deps: Vec<(String, String)> = Vec::new(); // (file_path, dep_name)

    for output_name in &output_names {
        let output_spec = match manifest.outputs.get(output_name) {
            Some(spec) => spec,
            None => continue,
        };
        for file_entry in &output_spec.files {
            for needed_file in &file_entry.needs {
                if let Some(dep_name) = manifest.resolution.get(needed_file) {
                    if dep_name == "self" {
                        // self lib - flatten from own package's files commit
                        self_libs.push(needed_file.clone());
                    } else {
                        // external dependency - resolve transitively
                        external_deps.push((needed_file.clone(), dep_name.clone()));
                    }
                }
            }
        }
    }

    // also collect transitive deps from self libs
    // (e.g., if libmount.so needs libblkid.so which is also self)
    for self_lib in &self_libs {
        let needs = find_file_needs(self_lib, manifest);
        for needed_file in needs {
            if let Some(dep_name) = manifest.resolution.get(&needed_file) {
                if dep_name == "self" {
                    if !self_libs.contains(&needed_file) {
                        // will be handled by dedup below
                    }
                } else {
                    external_deps.push((needed_file.clone(), dep_name.clone()));
                }
            }
        }
    }

    // dedup self_libs
    self_libs.sort();
    self_libs.dedup();

    let mut flattened_count = 0;
    let mut flattened_files: Vec<String> = Vec::new();

    // flatten self libs from own package's files commit
    if let Some(ref self_commit) = self_files_commit {
        for lib_path in &self_libs {
            if flatten_library_preserving_path(
                repo_path,
                self_commit,
                lib_path,
                pkg_dir,
                fallback_repos,
            )? {
                flattened_count += 1;
                flattened_files.push(lib_path.clone());
            }
        }
    }

    // resolve transitive closure of external dependencies
    if !external_deps.is_empty() {
        let all_deps = resolve_transitive_deps(&external_deps, manifest, manifest_index);
        for (file_path, _provider_key, provider_commit) in all_deps {
            if flatten_library_preserving_path(
                repo_path,
                &provider_commit,
                &file_path,
                pkg_dir,
                fallback_repos,
            )? {
                flattened_count += 1;
                flattened_files.push(file_path.clone());
            }
        }
    }

    // create lib/ld-linux-x86-64.so.2 symlink for nex-ld-shim loader lookup
    ensure_loader_symlink(pkg_dir, &flattened_files)?;

    Ok(flattened_count)
}

/// Resolve transitive closure of dependencies.
///
/// Starting from direct deps, recursively find all transitive deps by looking up
/// each dependency's manifest and checking what it needs.
///
/// Returns: Vec of (file_path, dep_name, files_commit)
fn resolve_transitive_deps(
    direct_deps: &[(String, String)], // (file_path, dep_name)
    root_manifest: &crate::manifest::types::Manifest,
    manifest_index: &ManifestIndex,
) -> Vec<(String, String, String)> {
    let mut result: Vec<(String, String, String)> = Vec::new();
    let mut seen_files: HashSet<String> = HashSet::new();
    // queue: (file_path, dep_name, source_manifest)
    let mut queue: Vec<(String, String, &crate::manifest::types::Manifest)> = Vec::new();

    // seed the queue with direct deps
    for (file_path, dep_name) in direct_deps {
        if seen_files.insert(file_path.clone()) {
            queue.push((file_path.clone(), dep_name.clone(), root_manifest));
        }
    }

    // cache for dep manifests we've looked up
    let mut dep_manifest_cache: HashMap<String, Option<&crate::manifest::types::Manifest>> =
        HashMap::new();

    while let Some((file_path, dep_name, source_manifest)) = queue.pop() {
        // handle __self: prefix for continuing resolution within same package
        let (actual_dep_name, is_self_continuation) = if dep_name.starts_with("__self:") {
            (dep_name.strip_prefix("__self:").unwrap().to_string(), true)
        } else {
            (dep_name.clone(), false)
        };

        // find the dependency by name in source manifest
        let dep = match find_dependency_by_name(source_manifest, &actual_dep_name) {
            Some(d) => d,
            None => continue,
        };

        // derive files commit for this dependency
        let files_commit = match derive_files_commit_for_dependency(dep, manifest_index) {
            Some(c) => c,
            None => continue, // can't derive files commit
        };

        // look up the dependency's manifest to get transitive deps
        let dep_manifest = if let Some(cached) = dep_manifest_cache.get(&actual_dep_name) {
            *cached
        } else {
            let m = find_manifest_for_dependency(dep, manifest_index);
            dep_manifest_cache.insert(actual_dep_name.clone(), m);
            m
        };

        if !is_self_continuation {
            let result_paths = dep_manifest
                .map(|m| expand_runtime_file_paths(&file_path, m))
                .unwrap_or_else(|| vec![file_path.clone()]);
            for result_path in result_paths {
                result.push((result_path, actual_dep_name.clone(), files_commit.clone()));
            }
        }

        let dep_manifest = match dep_manifest {
            Some(m) => m,
            None => continue, // can't find manifest, skip transitive deps
        };

        // find the file in the dependency's outputs to get its needs
        let file_needs = find_file_needs(&file_path, dep_manifest);

        // queue up transitive deps
        for needed_file in file_needs {
            if seen_files.insert(needed_file.clone()) {
                // resolve using dependency manifest's resolution map
                if let Some(transitive_dep_name) = dep_manifest.resolution.get(&needed_file) {
                    if transitive_dep_name == "self" {
                        // "self" means from the same package we're currently processing
                        // add to results and queue for further resolution using same dep context
                        for result_path in expand_runtime_file_paths(&needed_file, dep_manifest) {
                            result.push((
                                result_path,
                                actual_dep_name.clone(),
                                files_commit.clone(),
                            ));
                        }
                        // queue with source_manifest (which has this package as a dep), not dep_manifest
                        queue.push((
                            needed_file,
                            format!("__self:{}", actual_dep_name),
                            source_manifest,
                        ));
                    } else {
                        queue.push((needed_file, transitive_dep_name.clone(), dep_manifest));
                    }
                }
            }
        }
    }

    result
}

/// Expand runtime file paths that represent Python package imports.
///
/// A need such as `/usr/lib/python3.12/site-packages/requests/__init__.py`
/// means Python may import sibling modules from `requests/`. The capsule
/// flattener copies exact runtime paths, so it must copy the provider's whole
/// import package directory for package-style Python imports.
fn expand_runtime_file_paths(
    file_path: &str,
    manifest: &crate::manifest::types::Manifest,
) -> Vec<String> {
    let Some(prefix) = python_site_packages_dir_prefix(file_path) else {
        return vec![file_path.to_string()];
    };

    let mut paths = BTreeSet::new();
    paths.insert(file_path.to_string());

    for output_spec in manifest.outputs.values() {
        for file_entry in &output_spec.files {
            if file_entry.path.starts_with(&prefix) {
                paths.insert(file_entry.path.clone());
            }
        }
    }

    paths.into_iter().collect()
}

fn python_site_packages_dir_prefix(file_path: &str) -> Option<String> {
    let marker = "/site-packages/";
    let marker_index = file_path.find(marker)?;
    let package_start = marker_index + marker.len();
    let after_marker = &file_path[package_start..];
    let package_name = after_marker.split('/').next()?;

    if package_name.is_empty() || !after_marker.contains('/') {
        return None;
    }

    Some(format!("{}{}/", &file_path[..package_start], package_name))
}

/// Find a dependency by name in a manifest.
fn find_dependency_by_name<'a>(
    manifest: &'a crate::manifest::types::Manifest,
    name: &str,
) -> Option<&'a crate::manifest::types::Dependency> {
    manifest
        .dependencies
        .iter()
        .find(|d| d.name.as_deref() == Some(name))
}

/// Derive the files commit for the current manifest (for self libs).
/// Uses x86_64/pkg/{namespace}/{slug}/{version}/{checksum}/files for stable checksums.
fn derive_files_commit_for_manifest(manifest: &crate::manifest::types::Manifest) -> Option<String> {
    let has_stable_checksum =
        manifest.package.checksum.is_some() && manifest.package.stable_checksum.unwrap_or(true);

    let address_hash = if has_stable_checksum {
        manifest.package.checksum.clone()?
    } else {
        // use manifest's git blob SHA (input-addressed for bootstrap packages)
        let manifest_path = PathBuf::from(format!(
            "pkg/{}/{}.yaml",
            manifest.package.namespace, manifest.package.slug
        ));
        hash_file_content(&manifest_path).ok()?
    };

    Some(format!(
        "x86_64/pkg/{}/{}/{}/{}/files",
        manifest.package.namespace, manifest.package.slug, manifest.package.version, address_hash
    ))
}

/// Derive the files commit for a dependency.
/// Uses x86_64/pkg/{namespace}/{slug}/{version}/{checksum}/files for stable checksums.
fn derive_files_commit_for_dependency(
    dep: &crate::manifest::types::Dependency,
    manifest_index: &ManifestIndex,
) -> Option<String> {
    // parse the dependency commit to extract package info
    // e.g., "x86_64/pkg/libs/system/glibc/2.39/outputs/lib" -> namespace="libs/system", slug="glibc", version="2.39"
    let parts: Vec<&str> = dep.commit.split('/').collect();
    let pkg_idx = parts.iter().position(|&p| p == "pkg")?;
    let end_idx = parts
        .iter()
        .position(|&p| p == "outputs" || p == "bundles")?;

    if end_idx <= pkg_idx + 2 {
        return None;
    }

    let version = parts[end_idx - 1];
    let slug = parts[end_idx - 2];
    let namespace_path = parts[pkg_idx + 1..end_idx - 2].join("/");

    // look up the dependency's manifest
    let dep_manifest = manifest_index.get_manifest(&namespace_path, slug)?;

    // determine address hash: use checksum if stable, else use manifest git blob SHA
    let has_stable_checksum = dep_manifest.package.checksum.is_some()
        && dep_manifest.package.stable_checksum.unwrap_or(true);

    let address_hash = if has_stable_checksum {
        dep_manifest.package.checksum.clone().unwrap()
    } else {
        // use manifest's git blob SHA (input-addressed for bootstrap packages)
        let manifest_path = PathBuf::from(format!("pkg/{}/{}.yaml", namespace_path, slug));
        hash_file_content(&manifest_path).ok()?
    };

    Some(format!(
        "x86_64/pkg/{}/{}/{}/{}/files",
        namespace_path, slug, version, address_hash
    ))
}

/// Find the manifest for a dependency.
fn find_manifest_for_dependency<'a>(
    dep: &crate::manifest::types::Dependency,
    manifest_index: &'a ManifestIndex,
) -> Option<&'a crate::manifest::types::Manifest> {
    // parse the dependency commit to extract package info
    let parts: Vec<&str> = dep.commit.split('/').collect();
    let pkg_idx = parts.iter().position(|&p| p == "pkg")?;
    let end_idx = parts
        .iter()
        .position(|&p| p == "outputs" || p == "bundles")?;

    if end_idx <= pkg_idx + 2 {
        return None;
    }

    let slug = parts[end_idx - 2];
    let namespace_path = parts[pkg_idx + 1..end_idx - 2].join("/");

    manifest_index.get_manifest(&namespace_path, slug)
}

/// Find what a specific file needs by looking through the manifest's outputs.
fn find_file_needs(file_path: &str, manifest: &crate::manifest::types::Manifest) -> Vec<String> {
    for output_spec in manifest.outputs.values() {
        for file_entry in &output_spec.files {
            if file_entry.path == file_path {
                return file_entry.needs.clone();
            }
        }
    }
    Vec::new()
}

/// Detect which outputs from a manifest are present in a capsule directory.
/// Scans the capsule for files that match each output's file list.
fn detect_outputs_in_capsule(
    pkg_dir: &Path,
    manifest: &crate::manifest::types::Manifest,
) -> Vec<String> {
    let mut present_outputs = Vec::new();

    for (output_name, output_spec) in &manifest.outputs {
        // check if any file from this output exists in the capsule
        for file_entry in &output_spec.files {
            let file_path = pkg_dir.join(file_entry.path.trim_start_matches('/'));
            if file_path.exists() || file_path.symlink_metadata().is_ok() {
                present_outputs.push(output_name.clone());
                break; // found one, no need to check more files
            }
        }
    }

    present_outputs
}

/// Find the manifest that corresponds to a store commit ref.
fn find_manifest_for_commit<'a>(
    commit: &str,
    index: &'a ManifestIndex,
) -> Option<&'a crate::manifest::types::Manifest> {
    use crate::refs::PackageRef;

    let pkg_ref = PackageRef::parse(commit).ok()?;
    index.get_manifest(&pkg_ref.namespace, &pkg_ref.slug)
}

/// Flatten a single library from a store commit, preserving original path structure.
/// Uses direct export from blob store instead of full checkout.
fn flatten_library_preserving_path(
    repo_path: &str,
    commit: &str,
    lib_path: &str,
    pkg_dir: &Path,
    fallback_repos: &[PathBuf],
) -> io::Result<bool> {
    let rel_path = lib_path.trim_start_matches('/');
    let dest = pkg_dir.join(rel_path);

    if dest.exists() {
        return Ok(false);
    }

    // export directly from blob store (no full checkout needed)
    // commit is already "{hash}/files", lib_path is the path inside it
    match export_single_file(repo_path, commit, lib_path, &dest, fallback_repos) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    }

    // for symlinks, also export the target if it's relative
    if dest
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        if let Ok(link_target) = fs::read_link(&dest) {
            if !link_target.is_absolute() {
                let target_rel = dest
                    .parent()
                    .map(|p| p.join(&link_target))
                    .and_then(|p| p.strip_prefix(pkg_dir).ok().map(|s| s.to_path_buf()));

                if let Some(target_rel_path) = target_rel {
                    let target_dest = pkg_dir.join(&target_rel_path);
                    if !target_dest.exists() {
                        let target_src_path = format!("/{}", target_rel_path.display());
                        let _ = export_single_file(
                            repo_path,
                            commit,
                            &target_src_path,
                            &target_dest,
                            fallback_repos,
                        );
                    }
                }
            }
        }
    }

    Ok(true)
}

/// Export a single file from a commit using zub's export_path.
fn export_single_file(
    repo_path: &str,
    commit: &str,
    src_path: &str,
    dest: &Path,
    fallback_repos: &[PathBuf],
) -> io::Result<()> {
    use zub::Repo;

    let opts = zub::ops::ExportOptions {
        overwrite: true,
        hardlink: true,
        preserve_sparse: false,
    };

    // create parent directory if needed
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    // try primary repo
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    match zub::ops::export_path(&repo, commit, src_path, dest, opts.clone()) {
        Ok(()) => return Ok(()),
        Err(zub::Error::RefNotFound(_)) | Err(zub::Error::PathNotFound(_)) => {}
        Err(e) => return Err(io::Error::other(e.to_string())),
    }

    // try fallbacks
    for fallback_path in fallback_repos {
        if let Ok(fallback) = Repo::open(fallback_path) {
            match zub::ops::export_path(&fallback, commit, src_path, dest, opts.clone()) {
                Ok(()) => return Ok(()),
                Err(zub::Error::RefNotFound(_)) | Err(zub::Error::PathNotFound(_)) => continue,
                Err(e) => return Err(io::Error::other(e.to_string())),
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("path {} not found in commit {}", src_path, commit),
    ))
}

/// Create lib/ld-linux-x86-64.so.2 symlink for nex-ld-shim loader lookup.
fn ensure_loader_symlink(pkg_dir: &Path, flattened_files: &[String]) -> io::Result<()> {
    let loader_name = "ld-linux-x86-64.so.2";
    let has_loader = flattened_files.iter().any(|f| f.ends_with(loader_name));

    if !has_loader {
        return Ok(());
    }

    let lib_dir = pkg_dir.join("lib");
    let loader_symlink = lib_dir.join(loader_name);

    if loader_symlink.exists() || loader_symlink.symlink_metadata().is_ok() {
        return Ok(());
    }

    fs::create_dir_all(&lib_dir)?;
    symlink(format!("../usr/lib/{}", loader_name), &loader_symlink)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::python_site_packages_dir_prefix;

    #[test]
    fn finds_python_package_directory_prefix() {
        assert_eq!(
            python_site_packages_dir_prefix(
                "/usr/lib/python3.12/site-packages/requests/__init__.py"
            )
            .as_deref(),
            Some("/usr/lib/python3.12/site-packages/requests/")
        );
        assert_eq!(
            python_site_packages_dir_prefix(
                "/usr/lib/python3.12/site-packages/gi/_gi.cpython-312-x86_64-linux-gnu.so"
            )
            .as_deref(),
            Some("/usr/lib/python3.12/site-packages/gi/")
        );
    }

    #[test]
    fn ignores_top_level_python_module_files() {
        assert_eq!(
            python_site_packages_dir_prefix("/usr/lib/python3.12/site-packages/libvirt.py"),
            None
        );
        assert_eq!(
            python_site_packages_dir_prefix("/usr/lib/libvirt.so.0"),
            None
        );
    }
}
