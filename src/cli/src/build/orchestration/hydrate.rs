//! Dependency hydration for package manifests.

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::deps::resolve_dependency_closure;
use crate::manifest::types::Dependency;
use crate::manifest::{load_manifest, ManifestData, ManifestIndex};
use crate::refs::PackageRef;

/// Hydrate direct package dependencies with their transitive dependency closure.
pub fn hydrate_dependencies(
    _repo_path: &str,
    manifest_file: &str,
    manifest_dirs: &[PathBuf],
) -> io::Result<()> {
    let manifest_data = load_manifest(manifest_file)?;
    let dependencies = package_dependencies(&manifest_data)?;
    let manifest_index = ManifestIndex::load_many(manifest_dirs)?;
    let hydrated_deps = hydrated_dependencies(dependencies, &manifest_index)?;

    let content = fs::read_to_string(manifest_file)?;
    let output = replace_dependency_section(&content, &hydrated_deps)?;
    fs::write(manifest_file, output)?;

    println!(
        "Hydrated {} dependencies (was {})",
        hydrated_deps.len(),
        dependencies.len()
    );

    Ok(())
}

fn package_dependencies(manifest_data: &ManifestData) -> io::Result<&[Dependency]> {
    match manifest_data {
        ManifestData::Package(manifest) => Ok(&manifest.dependencies),
        ManifestData::System(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--hydrate-dependencies only applies to package manifests",
        )),
    }
}

fn hydrated_dependencies(
    dependencies: &[Dependency],
    manifest_index: &ManifestIndex,
) -> io::Result<Vec<Dependency>> {
    let all_commits = resolve_dependency_closure(dependencies, manifest_index)?;
    all_commits
        .into_iter()
        .map(|commit| {
            Ok(Dependency {
                name: hydrated_dependency_name(&commit, dependencies)?,
                commit,
                manifest_ref: None,
            })
        })
        .collect()
}

fn hydrated_dependency_name(
    commit: &str,
    direct_dependencies: &[Dependency],
) -> io::Result<Option<String>> {
    if let Some(name) = direct_dependencies
        .iter()
        .find(|dependency| dependency.commit == commit)
        .and_then(|dependency| dependency.name.clone())
    {
        return Ok(Some(name));
    }

    let package_ref = PackageRef::parse(commit).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid hydrated dependency ref {}: {}", commit, error),
        )
    })?;
    Ok(Some(package_ref.slug))
}

fn replace_dependency_section(content: &str, hydrated_deps: &[Dependency]) -> io::Result<String> {
    let lines: Vec<&str> = content.lines().collect();
    let (dep_start, dep_end) = dependency_section_bounds(&lines)?;
    let mut result: Vec<String> = lines[..dep_start].iter().map(|s| s.to_string()).collect();

    result.extend(format_dependency_section(hydrated_deps));
    result.extend(lines[dep_end..].iter().map(|s| s.to_string()));

    let output = result.join("\n");
    if content.ends_with('\n') {
        Ok(format!("{}\n", output))
    } else {
        Ok(output)
    }
}

fn dependency_section_bounds(lines: &[&str]) -> io::Result<(usize, usize)> {
    let mut dep_start = None;
    let mut dep_end = None;

    for (index, line) in lines.iter().enumerate() {
        if line.starts_with("dependencies:") {
            dep_start = Some(index);
        } else if dependency_section_ended(dep_start, dep_end, line) {
            dep_end = Some(index);
            break;
        }
    }

    let dep_start = dep_start.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "No dependencies section found in manifest",
        )
    })?;
    Ok((dep_start, dep_end.unwrap_or(lines.len())))
}

fn dependency_section_ended(dep_start: Option<usize>, dep_end: Option<usize>, line: &str) -> bool {
    dep_start.is_some()
        && dep_end.is_none()
        && !line.is_empty()
        && !line.starts_with(' ')
        && !line.starts_with('\t')
        && !line.starts_with('-')
}

fn format_dependency_section(hydrated_deps: &[Dependency]) -> Vec<String> {
    let mut section = vec!["dependencies:".to_string()];
    for dep in hydrated_deps {
        if let Some(name) = &dep.name {
            section.push(format!("  - name: {}", name));
            section.push(format!("    commit: {}", dep.commit));
        } else {
            section.push(format!("  - commit: {}", dep.commit));
        }
    }
    section
}

#[cfg(test)]
#[path = "hydrate_tests.rs"]
mod hydrate_tests;
