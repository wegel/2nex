//! Dependency-chain tracing for build plans.

use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::types::{Dependency, Manifest};
use crate::manifest::{load_manifest, ManifestData, ManifestIndex};
use crate::system;

/// Trace and display dependency chains that include a specific pattern.
pub fn trace_dependency_chains(
    _repo_path: &str,
    manifest_path: &Path,
    _manifest_dirs: &[PathBuf],
    pattern: &str,
) -> io::Result<()> {
    println!("Tracing dependencies matching pattern: '{}'", pattern);
    println!("Starting from: {}\n", manifest_path.display());

    let manifest_index = ManifestIndex::load("pkg")?;
    let (root_slug, root_deps) = root_dependency_list(manifest_path)?;
    let mut found_matches = false;

    for dep in &root_deps {
        let mut chain = vec![root_slug.clone()];
        if trace_commit_recursive(&dep.commit, pattern, &mut chain, &manifest_index)? {
            found_matches = true;
        }
    }

    if !found_matches {
        println!("No dependencies matching pattern '{}' found.", pattern);
    }

    Ok(())
}

fn root_dependency_list(manifest_path: &Path) -> io::Result<(String, Vec<Dependency>)> {
    let manifest_data = load_manifest(manifest_path.to_str().unwrap())?;
    Ok(match manifest_data {
        ManifestData::Package(manifest) => (manifest.package.slug.clone(), manifest.dependencies),
        ManifestData::System(manifest) => {
            let mut all_deps = manifest.dependencies.clone();
            all_deps.extend(system::dependencies_from_system_packages(
                &manifest.packages,
            ));
            (manifest.system.slug.clone(), all_deps)
        }
    })
}

fn trace_commit_recursive(
    commit: &str,
    pattern: &str,
    chain: &mut Vec<String>,
    manifest_index: &ManifestIndex,
) -> io::Result<bool> {
    chain.push(package_name_for_commit(commit));

    if commit.contains(pattern) {
        print_trace_match(commit, chain);
        chain.pop();
        return Ok(true);
    }

    let mut found_in_subtree = false;
    for dep_commit in get_manifest_deps(commit, manifest_index) {
        if trace_commit_recursive(&dep_commit, pattern, chain, manifest_index)? {
            found_in_subtree = true;
        }
    }

    chain.pop();
    Ok(found_in_subtree)
}

fn package_name_for_commit(commit: &str) -> String {
    match crate::refs::PackageRef::parse(commit) {
        Ok(pkg_ref) => format!("{}/{}/{}", pkg_ref.namespace, pkg_ref.slug, pkg_ref.version),
        Err(_) => commit.to_string(),
    }
}

fn print_trace_match(commit: &str, chain: &[String]) {
    println!("Found match: {}", commit);
    println!("  Chain: {}", chain.join(" -> "));
    println!();
}

fn get_manifest_deps(commit: &str, manifest_index: &ManifestIndex) -> Vec<String> {
    let Some((namespace, slug)) = package_parts_from_commit(commit) else {
        return Vec::new();
    };
    let Some(manifest) = manifest_index.get_manifest(&namespace, &slug) else {
        return Vec::new();
    };

    let mut deps = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for dep_name in manifest
        .resolution
        .values()
        .filter(|dep_name| *dep_name != "self")
    {
        if let Some(dep) = dependency_by_name(manifest, dep_name) {
            if seen.insert(dep.commit.clone()) {
                deps.push(dep.commit.clone());
            }
        }
    }
    deps
}

fn package_parts_from_commit(commit: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = commit.split('/').collect();
    let pkg_idx = parts.iter().position(|&part| part == "pkg")?;
    let end_idx = parts
        .iter()
        .position(|&part| part == "outputs" || part == "bundles")?;
    if end_idx <= pkg_idx + 2 {
        return None;
    }

    let slug_idx = end_idx - 2;
    Some((
        parts[pkg_idx + 1..slug_idx].join("/"),
        parts[slug_idx].to_string(),
    ))
}

fn dependency_by_name<'a>(manifest: &'a Manifest, dep_name: &str) -> Option<&'a Dependency> {
    manifest
        .dependencies
        .iter()
        .find(|dep| dep.name.as_deref() == Some(dep_name))
}
