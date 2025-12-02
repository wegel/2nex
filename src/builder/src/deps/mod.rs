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
    let mut stack = Vec::new();

    for dep in dependencies {
        visit_commit(
            &dep.commit,
            manifest_index,
            &mut seen,
            &mut visiting,
            &mut stack,
            &mut resolved,
        )?;
    }

    Ok(resolved)
}

/// visit a commit in dependency graph traversal (DFS with cycle detection).
fn visit_commit(
    commit: &str,
    manifest_index: &ManifestIndex,
    seen: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
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

    visiting.insert(commit.to_string());
    stack.push(commit.to_string());

    // fetch runtime dependencies from manifest
    let transitive_deps = fetch_deps_from_manifest(commit, manifest_index)?;

    for dep_commit in &transitive_deps {
        visit_commit(dep_commit, manifest_index, seen, visiting, stack, resolved)?;
    }

    stack.pop();
    visiting.remove(commit);
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
    // parse commit: x86_64/pkg/namespace/slug/version/outputs/name or .../bundles/name
    let parts: Vec<&str> = commit.split('/').collect();

    // find "pkg" position
    let pkg_idx = match parts.iter().position(|&p| p == "pkg") {
        Some(idx) => idx,
        None => return Ok(Vec::new()), // not a package commit, no deps
    };

    // find "outputs" or "bundles" position
    let end_idx = match parts.iter().position(|&p| p == "outputs" || p == "bundles") {
        Some(idx) => idx,
        None => return Ok(Vec::new()), // malformed commit ref
    };

    if end_idx <= pkg_idx + 2 {
        return Ok(Vec::new()); // not enough parts
    }

    // extract namespace, slug, version
    let version_idx = end_idx - 1;
    let slug_idx = version_idx - 1;

    if slug_idx <= pkg_idx {
        return Ok(Vec::new());
    }

    let namespace = parts[pkg_idx + 1..slug_idx].join("/");
    let slug = parts[slug_idx];
    let commit_type = parts.get(end_idx).copied().unwrap_or("");
    let commit_name = parts.get(end_idx + 1).copied().unwrap_or("");

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
