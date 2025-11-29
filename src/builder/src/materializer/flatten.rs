//! Dependency flattening for Nex capsules using precomputed deps.
//!
//! This module handles flattening runtime dependencies into each package's
//! `lib/` directory, creating self-contained "capsules" that nex-ld-shim can use.
//!
//! Dependencies are resolved transitively: if binary A needs libB.so, and libB.so
//! needs libC.so, we flatten both libB.so and libC.so into A's capsule.
//!
//! The resolution map points file paths to dependency names (or @self for internal).
//! The files commit is derived from the dependency's manifest.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::manifest::ManifestIndex;
use crate::ostree_native::OstreeRepo;
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
) -> io::Result<usize> {
    // find manifest for this commit
    let manifest = match find_manifest_for_commit(commit, manifest_index) {
        Some(m) => m,
        None => {
            // no manifest - can't flatten without precomputed deps
            return Ok(0);
        }
    };

    // get the current package's files commit for @self libs
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
            None => return Ok(0), // bundle not in manifest
        }
    } else {
        // single output specified, but check for other outputs in the capsule
        // this handles the case where system assembly merges multiple outputs
        detect_outputs_in_capsule(pkg_dir, manifest)
    };

    // collect all needed libs (both external deps and @self libs)
    // for @self libs, we flatten from own {checksum}/files commit
    // for external deps, we resolve transitively
    let mut self_libs: Vec<String> = Vec::new(); // file paths for @self libs
    let mut external_deps: Vec<(String, String)> = Vec::new(); // (file_path, dep_name)

    for output_name in &output_names {
        let output_spec = match manifest.outputs.get(output_name) {
            Some(spec) => spec,
            None => continue,
        };
        for file_entry in &output_spec.files {
            for needed_file in &file_entry.needs {
                if let Some(dep_name) = manifest.resolution.get(needed_file) {
                    if dep_name == "@self" {
                        // @self lib - flatten from own package's files commit
                        self_libs.push(needed_file.clone());
                    } else {
                        // external dependency - resolve transitively
                        external_deps.push((needed_file.clone(), dep_name.clone()));
                    }
                }
            }
        }
    }

    // also collect transitive deps from @self libs
    // (e.g., if libmount.so needs libblkid.so which is also @self)
    for self_lib in &self_libs {
        let needs = find_file_needs(self_lib, manifest);
        for needed_file in needs {
            if let Some(dep_name) = manifest.resolution.get(&needed_file) {
                if dep_name == "@self" {
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

    let pkg_lib_dir = pkg_dir.join("lib");
    fs::create_dir_all(&pkg_lib_dir)?;

    let mut flattened_count = 0;

    // flatten @self libs from own package's files commit
    if let Some(ref self_commit) = self_files_commit {
        for lib_path in &self_libs {
            if flatten_library_from_commit(repo_path, self_commit, lib_path, &pkg_lib_dir)? {
                flattened_count += 1;
            }
        }
    }

    // resolve transitive closure of external dependencies
    if !external_deps.is_empty() {
        let all_deps = resolve_transitive_deps(&external_deps, manifest, manifest_index);
        for (file_path, _provider_key, provider_commit) in all_deps {
            if flatten_library_from_commit(repo_path, &provider_commit, &file_path, &pkg_lib_dir)? {
                flattened_count += 1;
            }
        }
    }

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
        // find the dependency by name in source manifest
        let dep = match find_dependency_by_name(source_manifest, &dep_name) {
            Some(d) => d,
            None => continue, // dependency not found
        };

        // derive files commit for this dependency
        let files_commit = match derive_files_commit_for_dependency(dep, manifest_index) {
            Some(c) => c,
            None => continue, // can't derive files commit
        };

        result.push((file_path.clone(), dep_name.clone(), files_commit));

        // look up the dependency's manifest to get transitive deps
        let dep_manifest = if let Some(cached) = dep_manifest_cache.get(&dep_name) {
            *cached
        } else {
            let m = find_manifest_for_dependency(dep, manifest_index);
            dep_manifest_cache.insert(dep_name.clone(), m);
            m
        };

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
                    // skip @self entries
                    if transitive_dep_name != "@self" {
                        queue.push((needed_file, transitive_dep_name.clone(), dep_manifest));
                    }
                }
            }
        }
    }

    result
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

/// Derive the files commit for the current manifest (for @self libs).
/// Uses {checksum}/files for stable checksums, {git_blob_sha}/files for bootstrap packages.
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

    Some(format!("{}/files", address_hash))
}

/// Derive the files commit for a dependency.
/// Uses {checksum}/files for stable checksums, {git_blob_sha}/files for bootstrap packages.
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

    Some(format!("{}/files", address_hash))
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

/// Find the manifest that corresponds to an OSTree commit ref.
fn find_manifest_for_commit<'a>(
    commit: &str,
    index: &'a ManifestIndex,
) -> Option<&'a crate::manifest::types::Manifest> {
    let parts: Vec<&str> = commit.split('/').collect();
    let pkg_idx = parts.iter().position(|&p| p == "pkg")?;
    let end_idx = parts
        .iter()
        .position(|&p| p == "outputs" || p == "bundles")?;

    if end_idx <= pkg_idx + 2 {
        return None;
    }

    let version_idx = end_idx - 1;
    let slug_idx = version_idx - 1;

    if slug_idx <= pkg_idx {
        return None;
    }

    let namespace = parts[pkg_idx + 1..slug_idx].join("/");
    let slug = parts[slug_idx];

    index.get_manifest(&namespace, slug)
}

/// Flatten a single library from an OSTree commit into a package's lib/ directory.
fn flatten_library_from_commit(
    repo_path: &str,
    commit: &str,
    lib_path: &str,
    pkg_lib_dir: &Path,
) -> io::Result<bool> {
    let basename = Path::new(lib_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid library path"))?;

    let dest = pkg_lib_dir.join(basename);
    if dest.exists() {
        return Ok(false); // already flattened
    }

    // checkout the whole commit to extract the library
    // must be on same filesystem as repo for hardlinks to work
    let staging_dir = Path::new("/nex/staging");
    let temp_parent = if staging_dir.exists() || fs::create_dir_all(staging_dir).is_ok() {
        staging_dir
    } else {
        Path::new(repo_path).parent().unwrap_or(Path::new("."))
    };
    let temp = TempDir::new_in(temp_parent)?;
    let checkout_dir = temp.path().join("checkout");

    let repo = OstreeRepo::open(repo_path)?;
    repo.checkout(commit, &checkout_dir, true)?;

    let src = checkout_dir.join(lib_path.trim_start_matches('/'));
    if !src.exists() {
        return Ok(false);
    }

    fs::create_dir_all(pkg_lib_dir)?;
    hardlink_or_symlink(&src, &dest)?;

    // handle symlink targets (e.g., libc.so.6 -> libc-2.39.so)
    if src.symlink_metadata()?.file_type().is_symlink() {
        let link_target = fs::read_link(&src)?;
        if !link_target.is_absolute() {
            let target_name = link_target.file_name().and_then(|n| n.to_str());
            if let Some(target_name) = target_name {
                let target_src = src.parent().unwrap().join(&link_target);
                let target_dest = pkg_lib_dir.join(target_name);
                if target_src.exists() && !target_dest.exists() {
                    hardlink_or_symlink(&target_src, &target_dest)?;
                }
            }
        }
    }

    Ok(true)
}

/// Hardlink a file or recreate a symlink.
fn hardlink_or_symlink(src: &Path, dest: &Path) -> io::Result<()> {
    let meta = src.symlink_metadata()?;

    if meta.file_type().is_symlink() {
        let target = fs::read_link(src)?;
        if dest.exists() || dest.symlink_metadata().is_ok() {
            fs::remove_file(dest)?;
        }
        symlink(&target, dest)?;
    } else {
        if dest.exists() || dest.symlink_metadata().is_ok() {
            fs::remove_file(dest)?;
        }
        fs::hard_link(src, dest).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "failed to hardlink {} -> {}: {} (cross-device mounts not supported)",
                    src.display(),
                    dest.display(),
                    e
                ),
            )
        })?;
    }

    Ok(())
}
