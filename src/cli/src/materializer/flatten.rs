//! Dependency flattening for Nex capsules using precomputed deps.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use crate::manifest::types::Manifest;
use crate::manifest::ManifestIndex;

use super::flatten_deps::{find_file_needs, resolve_transitive_deps};
use super::flatten_errors::{
    missing_bundle_error, missing_file_metadata_error, missing_manifest_error,
    missing_output_error, missing_resolution_error, missing_self_files_commit_error,
};
use super::flatten_export::flatten_library_preserving_path;
use super::flatten_refs::derive_files_commit_for_manifest;

#[cfg(test)]
#[path = "flatten_export_tests.rs"]
mod flatten_export_tests;

#[cfg(test)]
#[path = "flatten_runtime_tests.rs"]
mod flatten_runtime_tests;

/// Flatten runtime dependencies for a single package capsule using precomputed deps.
///
/// Reads deps from the manifest and copies required libraries into the package capsule.
pub fn flatten_capsule_precomputed(
    repo_path: &str,
    pkg_dir: &Path,
    commit: &str,
    manifest_index: &ManifestIndex,
    fallback_repos: &[PathBuf],
) -> io::Result<usize> {
    let manifest = match find_manifest_for_commit(commit, manifest_index) {
        Some(manifest) => manifest,
        None => return Err(missing_manifest_error(commit)),
    };
    let self_files_commit = derive_files_commit_for_manifest(manifest);
    let output_names = output_names_for_commit(pkg_dir, commit, manifest)?;
    let deps = collect_capsule_deps(&output_names, manifest)?;

    let mut flattened_files = Vec::new();
    flatten_self_libs(
        repo_path,
        pkg_dir,
        fallback_repos,
        self_files_commit.as_deref(),
        manifest,
        &deps.self_libs,
        &mut flattened_files,
    )?;
    flatten_external_libs(
        repo_path,
        pkg_dir,
        fallback_repos,
        manifest,
        manifest_index,
        &deps.external_deps,
        &mut flattened_files,
    )?;
    ensure_loader_symlink(pkg_dir, &flattened_files)?;

    Ok(flattened_files.len())
}

struct CapsuleDeps {
    self_libs: Vec<String>,
    external_deps: Vec<(String, String)>,
}

fn output_names_for_commit(
    pkg_dir: &Path,
    commit: &str,
    manifest: &crate::manifest::types::Manifest,
) -> io::Result<Vec<String>> {
    let commit_parts: Vec<&str> = commit.split('/').collect();
    let commit_type = commit_parts
        .get(commit_parts.len().saturating_sub(2))
        .copied()
        .unwrap_or("");
    let commit_name = commit_parts.last().copied().unwrap_or("");

    if commit_type == "bundles" {
        let Some(bundle) = manifest.bundles.get(commit_name) else {
            return Err(missing_bundle_error(commit_name, manifest));
        };
        Ok(bundle.includes.clone())
    } else if commit_type == "outputs" {
        Ok(vec![commit_name.to_string()])
    } else {
        Ok(detect_outputs_in_capsule(pkg_dir, manifest))
    }
}

fn collect_capsule_deps(output_names: &[String], manifest: &Manifest) -> io::Result<CapsuleDeps> {
    let mut deps = CapsuleDeps {
        self_libs: Vec::new(),
        external_deps: Vec::new(),
    };

    for output_name in output_names {
        collect_output_deps(output_name, manifest, &mut deps)?;
    }
    collect_external_deps_from_self_libs(manifest, &mut deps)?;
    deps.self_libs.sort();
    deps.self_libs.dedup();
    Ok(deps)
}

fn collect_output_deps(
    output_name: &str,
    manifest: &Manifest,
    deps: &mut CapsuleDeps,
) -> io::Result<()> {
    let Some(output_spec) = manifest.outputs.get(output_name) else {
        return Err(missing_output_error(output_name, manifest));
    };

    for file_entry in &output_spec.files {
        for needed_file in &file_entry.needs {
            collect_needed_file(needed_file, manifest, deps)?;
        }
    }
    Ok(())
}

fn collect_needed_file(
    needed_file: &str,
    manifest: &Manifest,
    deps: &mut CapsuleDeps,
) -> io::Result<()> {
    let Some(dep_name) = manifest.resolution.get(needed_file) else {
        return Err(missing_resolution_error(needed_file, manifest));
    };
    if dep_name == "self" {
        deps.self_libs.push(needed_file.to_string());
    } else {
        deps.external_deps
            .push((needed_file.to_string(), dep_name.clone()));
    }
    Ok(())
}

fn collect_external_deps_from_self_libs(
    manifest: &Manifest,
    deps: &mut CapsuleDeps,
) -> io::Result<()> {
    let mut seen_self_libs = deps.self_libs.iter().cloned().collect::<BTreeSet<_>>();
    let mut index = 0;
    while index < deps.self_libs.len() {
        let self_lib = deps.self_libs[index].clone();
        index += 1;
        let Some(needs) = find_file_needs(&self_lib, manifest) else {
            return Err(missing_file_metadata_error(&self_lib, manifest));
        };
        for needed_file in needs {
            let Some(dep_name) = manifest.resolution.get(&needed_file) else {
                return Err(missing_resolution_error(&needed_file, manifest));
            };
            if dep_name == "self" {
                if seen_self_libs.insert(needed_file.clone()) {
                    deps.self_libs.push(needed_file);
                }
            } else {
                deps.external_deps.push((needed_file, dep_name.clone()));
            }
        }
    }
    Ok(())
}

fn flatten_self_libs(
    repo_path: &str,
    pkg_dir: &Path,
    fallback_repos: &[PathBuf],
    self_files_commit: Option<&str>,
    manifest: &Manifest,
    self_libs: &[String],
    flattened_files: &mut Vec<String>,
) -> io::Result<()> {
    if self_libs.is_empty() {
        return Ok(());
    }
    let Some(self_commit) = self_files_commit else {
        return Err(missing_self_files_commit_error(manifest, self_libs));
    };
    for lib_path in self_libs {
        if flatten_library_preserving_path(
            repo_path,
            self_commit,
            lib_path,
            pkg_dir,
            fallback_repos,
        )? {
            flattened_files.push(lib_path.clone());
        }
    }
    Ok(())
}

fn flatten_external_libs(
    repo_path: &str,
    pkg_dir: &Path,
    fallback_repos: &[PathBuf],
    manifest: &Manifest,
    manifest_index: &ManifestIndex,
    external_deps: &[(String, String)],
    flattened_files: &mut Vec<String>,
) -> io::Result<()> {
    for (file_path, _provider_key, provider_commit) in
        resolve_transitive_deps(external_deps, manifest, manifest_index)?
    {
        if flatten_library_preserving_path(
            repo_path,
            &provider_commit,
            &file_path,
            pkg_dir,
            fallback_repos,
        )? {
            flattened_files.push(file_path);
        }
    }
    Ok(())
}

/// Detect which outputs from a manifest are present in a capsule directory.
/// Scans the capsule for files that match each output's file list.
fn detect_outputs_in_capsule(pkg_dir: &Path, manifest: &Manifest) -> Vec<String> {
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
fn find_manifest_for_commit<'a>(commit: &str, index: &'a ManifestIndex) -> Option<&'a Manifest> {
    use crate::refs::PackageRef;

    let pkg_ref = PackageRef::parse(commit).ok()?;
    index.get_manifest(&pkg_ref.namespace, &pkg_ref.slug)
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
