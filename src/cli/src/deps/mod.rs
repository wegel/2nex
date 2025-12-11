//! dependency closure resolution using manifest-based `needs`/`resolution`.
//!
//! this module resolves transitive dependencies by reading precomputed dependency
//! information from package manifests instead of store metadata.

use std::collections::HashSet;
use std::io;

use crate::manifest::{Dependency, ManifestIndex};

/// resolve the full dependency closure using manifests.
///
/// for each dependency commit, finds its manifest and resolves all runtime
/// dependencies through the `needs` and `resolution` fields.
///
/// returns commits in dependency order (dependencies before dependents).
pub fn resolve_dependency_closure(
    dependencies: &[Dependency],
    manifest_index: &ManifestIndex,
) -> io::Result<Vec<String>> {
    let mut resolved = Vec::new();
    let mut seen = HashSet::new();
    let mut visiting = HashSet::new();
    let mut visiting_packages = HashSet::new();
    let mut stack = Vec::new();

    for dep in dependencies {
        visit_commit(
            &dep.commit,
            manifest_index,
            &mut seen,
            &mut visiting,
            &mut visiting_packages,
            &mut stack,
            &mut resolved,
        )?;
    }

    Ok(resolved)
}

/// extract package identity (namespace, slug) from a commit ref.
fn get_package_identity(commit: &str) -> Option<(String, String)> {
    use crate::refs::PackageRef;
    PackageRef::parse(commit)
        .ok()
        .map(|r| (r.namespace, r.slug))
}

/// visit a commit in dependency graph traversal (DFS with cycle detection).
fn visit_commit(
    commit: &str,
    manifest_index: &ManifestIndex,
    seen: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
    visiting_packages: &mut HashSet<(String, String)>,
    stack: &mut Vec<String>,
    resolved: &mut Vec<String>,
) -> io::Result<()> {
    if seen.contains(commit) {
        return Ok(());
    }

    if visiting.contains(commit) {
        let cycle_path = build_cycle_path(stack, commit);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("circular dependency detected: {}", cycle_path),
        ));
    }

    // check for package identity cycle (same namespace/slug, different version)
    let identity = get_package_identity(commit);
    if let Some((ref ns, ref slug)) = identity {
        if visiting_packages.contains(&(ns.clone(), slug.clone())) {
            let cycle_path = build_cycle_path(stack, commit);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "package cycle: {}/{} appears in its own dependency chain: {}",
                    ns, slug, cycle_path
                ),
            ));
        }
        visiting_packages.insert((ns.clone(), slug.clone()));
    }

    visiting.insert(commit.to_string());
    stack.push(commit.to_string());

    // fetch runtime dependencies from manifest
    let transitive_deps = fetch_deps_from_manifest(commit, manifest_index)?;

    for dep_commit in &transitive_deps {
        visit_commit(
            dep_commit,
            manifest_index,
            seen,
            visiting,
            visiting_packages,
            stack,
            resolved,
        )?;
    }

    stack.pop();
    visiting.remove(commit);
    if let Some((ns, slug)) = identity {
        visiting_packages.remove(&(ns, slug));
    }
    seen.insert(commit.to_string());
    resolved.push(commit.to_string());

    Ok(())
}

/// fetch runtime dependencies for a commit using manifest `needs`/`resolution`.
///
/// parses the commit ref to find the manifest, then collects all `needs` entries
/// from the relevant outputs and resolves them through the `resolution` map.
fn fetch_deps_from_manifest(
    commit: &str,
    manifest_index: &ManifestIndex,
) -> io::Result<Vec<String>> {
    use crate::refs::{PackageRef, RefType};

    let pkg_ref = match PackageRef::parse(commit) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()), // not a valid package ref, no deps
    };

    // files refs don't have runtime deps to resolve (they're raw file extractions)
    if pkg_ref.is_files() {
        return Ok(Vec::new());
    }

    let namespace = &pkg_ref.namespace;
    let slug = &pkg_ref.slug;

    // get output/bundle name based on ref type
    let (commit_type, commit_name) = match &pkg_ref.ref_type {
        RefType::Output { name, .. } => ("outputs", name.as_str()),
        RefType::Bundle { name, .. } => ("bundles", name.as_str()),
        RefType::Files { .. } => return Ok(Vec::new()),
    };

    // find manifest
    let manifest = match manifest_index.get_manifest(&namespace, slug) {
        Some(m) => m,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "manifest not found for {}/{} (commit: {}). run `nex compute-deps` first.",
                    namespace, slug, commit
                ),
            ));
        }
    };

    // determine which outputs to process
    let output_names: Vec<String> = if commit_type == "bundles" {
        match manifest.bundles.get(commit_name) {
            Some(bundle) => bundle.includes.clone(),
            None => return Ok(Vec::new()),
        }
    } else {
        vec![commit_name.to_string()]
    };

    // collect all needed dependencies
    let mut deps = Vec::new();
    let mut seen_deps = HashSet::new();

    for output_name in &output_names {
        let output_spec = match manifest.outputs.get(output_name) {
            Some(spec) => spec,
            None => continue,
        };

        for file_entry in &output_spec.files {
            for needed_file in &file_entry.needs {
                // resolve file path to dependency name via resolution map
                let dep_name = match manifest.resolution.get(needed_file) {
                    Some(name) => name,
                    None => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "{}: file {} needs {} but no resolution found. run `nex compute-deps`.",
                                commit, file_entry.path, needed_file
                            ),
                        ));
                    }
                };

                // skip self entries - internal libs don't need resolution
                if dep_name == "self" {
                    continue;
                }

                // find the dependency commit from the manifest's dependencies
                let dep_commit = match manifest
                    .dependencies
                    .iter()
                    .find(|d| d.name.as_deref() == Some(dep_name))
                {
                    Some(d) => d.commit.clone(),
                    None => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "{}: resolution refers to '{}' but no dependency with that name found",
                                commit, dep_name
                            ),
                        ));
                    }
                };

                if seen_deps.insert(dep_commit.clone()) {
                    deps.push(dep_commit);
                }
            }
        }
    }

    Ok(deps)
}

/// build a human-readable cycle path string.
fn build_cycle_path(stack: &[String], repeat: &str) -> String {
    let mut path = stack.to_vec();
    path.push(repeat.to_string());
    path.join(" -> ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_dependencies() {
        let index = ManifestIndex::new();
        let deps: Vec<Dependency> = vec![];
        let result = resolve_dependency_closure(&deps, &index).unwrap();
        assert!(result.is_empty());
    }
}
