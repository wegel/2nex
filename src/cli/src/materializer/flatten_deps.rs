//! Transitive runtime dependency discovery for capsule flattening.

use std::collections::{BTreeSet, HashSet};
use std::io;
use std::path::PathBuf;

use crate::manifest::types::Manifest;
use crate::manifest::ManifestIndex;
use crate::utils::hash_file_content;

use super::flatten_errors::{
    missing_dependency_error, missing_dependency_files_commit_error, missing_file_metadata_error,
    missing_resolution_error,
};

/// Resolve transitive closure of dependencies.
pub(super) fn resolve_transitive_deps(
    direct_deps: &[(String, String)],
    root_manifest: &Manifest,
    manifest_index: &ManifestIndex,
) -> io::Result<Vec<(String, String, String)>> {
    let mut result: Vec<(String, String, String)> = Vec::new();
    let (mut seen_files, mut queue) = seed_dependency_queue(direct_deps, root_manifest);

    while let Some((file_path, dep_name, source_manifest)) = queue.pop() {
        process_dependency_queue_item(
            file_path,
            dep_name,
            source_manifest,
            manifest_index,
            &mut seen_files,
            &mut queue,
            &mut result,
        )?;
    }

    Ok(result)
}

type DependencyQueue<'a> = Vec<(String, String, &'a Manifest)>;

fn seed_dependency_queue<'a>(
    direct_deps: &[(String, String)],
    root_manifest: &'a Manifest,
) -> (HashSet<String>, DependencyQueue<'a>) {
    let mut seen_files = HashSet::new();
    let mut queue = Vec::new();
    for (file_path, dep_name) in direct_deps {
        if seen_files.insert(file_path.clone()) {
            queue.push((file_path.clone(), dep_name.clone(), root_manifest));
        }
    }
    (seen_files, queue)
}

#[allow(clippy::too_many_arguments)]
fn process_dependency_queue_item<'a>(
    file_path: String,
    dep_name: String,
    source_manifest: &'a Manifest,
    manifest_index: &'a ManifestIndex,
    seen_files: &mut HashSet<String>,
    queue: &mut DependencyQueue<'a>,
    result: &mut Vec<(String, String, String)>,
) -> io::Result<()> {
    let (actual_dep_name, is_self_continuation) = actual_dependency_name(&dep_name);
    let Some(dep) = find_dependency_by_name(source_manifest, &actual_dep_name) else {
        return Err(missing_dependency_error(
            &file_path,
            &actual_dep_name,
            source_manifest,
        ));
    };
    let Some(files_commit) = derive_files_commit_for_dependency(dep, manifest_index) else {
        return Err(missing_dependency_files_commit_error(
            &file_path,
            &actual_dep_name,
            dep,
        ));
    };
    let dep_manifest = find_manifest_for_dependency(dep, manifest_index);

    if !is_self_continuation {
        push_result_paths(
            result,
            &file_path,
            &actual_dep_name,
            &files_commit,
            dep_manifest,
        );
    }

    if let Some(dep_manifest) = dep_manifest {
        queue_transitive_needs(
            &file_path,
            &actual_dep_name,
            source_manifest,
            dep_manifest,
            &files_commit,
            seen_files,
            queue,
            result,
        )?;
    }
    Ok(())
}

fn actual_dependency_name(dep_name: &str) -> (String, bool) {
    if let Some(dep_name) = dep_name.strip_prefix("__self:") {
        (dep_name.to_string(), true)
    } else {
        (dep_name.to_string(), false)
    }
}

/// Find what a specific file needs by looking through the manifest's outputs.
pub(super) fn find_file_needs(file_path: &str, manifest: &Manifest) -> Option<Vec<String>> {
    for output_spec in manifest.outputs.values() {
        for file_entry in &output_spec.files {
            if file_entry.path == file_path {
                return Some(file_entry.needs.clone());
            }
        }
    }
    None
}

pub(super) fn python_site_packages_dir_prefix(file_path: &str) -> Option<String> {
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

fn push_result_paths(
    result: &mut Vec<(String, String, String)>,
    file_path: &str,
    dep_name: &str,
    files_commit: &str,
    dep_manifest: Option<&Manifest>,
) {
    let result_paths = dep_manifest
        .map(|manifest| expand_runtime_file_paths(file_path, manifest))
        .unwrap_or_else(|| vec![file_path.to_string()]);
    for result_path in result_paths {
        result.push((result_path, dep_name.to_string(), files_commit.to_string()));
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_transitive_needs<'a>(
    file_path: &str,
    dep_name: &str,
    source_manifest: &'a Manifest,
    dep_manifest: &'a Manifest,
    files_commit: &str,
    seen_files: &mut HashSet<String>,
    queue: &mut Vec<(String, String, &'a Manifest)>,
    result: &mut Vec<(String, String, String)>,
) -> io::Result<()> {
    let Some(needed_files) = find_file_needs(file_path, dep_manifest) else {
        return Err(missing_file_metadata_error(file_path, dep_manifest));
    };
    for needed_file in needed_files {
        if !seen_files.insert(needed_file.clone()) {
            continue;
        }
        let Some(transitive_dep_name) = dep_manifest.resolution.get(&needed_file) else {
            return Err(missing_resolution_error(&needed_file, dep_manifest));
        };
        queue_one_need(
            needed_file,
            transitive_dep_name,
            dep_name,
            source_manifest,
            dep_manifest,
            files_commit,
            queue,
            result,
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn queue_one_need<'a>(
    needed_file: String,
    transitive_dep_name: &str,
    dep_name: &str,
    source_manifest: &'a Manifest,
    dep_manifest: &'a Manifest,
    files_commit: &str,
    queue: &mut Vec<(String, String, &'a Manifest)>,
    result: &mut Vec<(String, String, String)>,
) {
    if transitive_dep_name == "self" {
        for result_path in expand_runtime_file_paths(&needed_file, dep_manifest) {
            result.push((result_path, dep_name.to_string(), files_commit.to_string()));
        }
        queue.push((needed_file, format!("__self:{}", dep_name), source_manifest));
    } else {
        queue.push((needed_file, transitive_dep_name.to_string(), dep_manifest));
    }
}

fn expand_runtime_file_paths(file_path: &str, manifest: &Manifest) -> Vec<String> {
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

fn find_dependency_by_name<'a>(
    manifest: &'a Manifest,
    name: &str,
) -> Option<&'a crate::manifest::types::Dependency> {
    manifest
        .dependencies
        .iter()
        .find(|dep| dep.name.as_deref() == Some(name))
}

fn derive_files_commit_for_dependency(
    dep: &crate::manifest::types::Dependency,
    manifest_index: &ManifestIndex,
) -> Option<String> {
    let package_ref = dependency_package_ref(&dep.commit)?;
    let dep_manifest =
        manifest_index.get_manifest(&package_ref.namespace_path, &package_ref.slug)?;
    let address_hash = dependency_address_hash(dep_manifest, &package_ref)?;
    Some(package_ref.files_ref(&address_hash))
}

fn find_manifest_for_dependency<'a>(
    dep: &crate::manifest::types::Dependency,
    manifest_index: &'a ManifestIndex,
) -> Option<&'a Manifest> {
    let package_ref = dependency_package_ref(&dep.commit)?;
    manifest_index.get_manifest(&package_ref.namespace_path, &package_ref.slug)
}

struct DependencyPackageRef {
    namespace_path: String,
    slug: String,
    version: String,
}

impl DependencyPackageRef {
    fn files_ref(&self, address_hash: &str) -> String {
        format!(
            "x86_64/{}/{}/{}/{}/files",
            self.namespace_path, self.slug, self.version, address_hash
        )
    }
}

fn dependency_package_ref(commit: &str) -> Option<DependencyPackageRef> {
    let parts: Vec<&str> = commit.split('/').collect();
    let pkg_idx = parts.iter().position(|&part| part == "pkg")?;
    let end_idx = parts
        .iter()
        .position(|&part| part == "outputs" || part == "bundles")?;
    if end_idx <= pkg_idx + 2 {
        return None;
    }

    Some(DependencyPackageRef {
        namespace_path: parts[pkg_idx..end_idx - 2].join("/"),
        slug: parts[end_idx - 2].to_string(),
        version: parts[end_idx - 1].to_string(),
    })
}

fn dependency_address_hash(
    manifest: &Manifest,
    package_ref: &DependencyPackageRef,
) -> Option<String> {
    let has_stable_checksum =
        manifest.package.checksum.is_some() && manifest.package.stable_checksum.unwrap_or(true);
    if has_stable_checksum {
        return manifest.package.checksum.clone();
    }

    let manifest_path = PathBuf::from(format!(
        "{}/{}.yaml",
        package_ref.namespace_path, package_ref.slug
    ));
    hash_file_content(&manifest_path).ok()
}

#[cfg(test)]
#[path = "flatten_tests.rs"]
mod flatten_tests;
