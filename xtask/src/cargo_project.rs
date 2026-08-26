//! Cargo project discovery and staged-file scope selection.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use crate::config::RustProjectConfig;

#[derive(Clone, Debug)]
pub(crate) struct RustProject {
    pub(crate) name: String,
    pub(crate) manifest_path: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) packages: Vec<RustPackage>,
    pub(crate) clippy_args: Vec<String>,
    pub(crate) test_args: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct RustPackage {
    pub(crate) name: String,
    pub(crate) root: PathBuf,
    pub(crate) library_root: Option<PathBuf>,
    pub(crate) non_library_entrypoints: BTreeSet<PathBuf>,
    pub(crate) dependencies: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CargoScope {
    Workspace,
    Packages(BTreeSet<String>),
}

#[derive(Debug, Deserialize)]
struct Metadata {
    workspace_root: PathBuf,
    packages: Vec<MetadataPackage>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MetadataPackage {
    id: String,
    name: String,
    manifest_path: PathBuf,
    targets: Vec<MetadataTarget>,
    dependencies: Vec<MetadataDependency>,
}

#[derive(Debug, Deserialize)]
struct MetadataTarget {
    kind: Vec<String>,
    src_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct MetadataDependency {
    path: Option<PathBuf>,
}

pub(crate) fn discover_projects(
    repo_root: &Path,
    configs: &[RustProjectConfig],
) -> Result<Vec<RustProject>> {
    configs
        .iter()
        .map(|config| discover_project(repo_root, config))
        .collect()
}

fn discover_project(repo_root: &Path, config: &RustProjectConfig) -> Result<RustProject> {
    let metadata = read_metadata(repo_root, config)?;
    let root = relative_to_repo(repo_root, &metadata.workspace_root)?;
    let packages = workspace_packages(repo_root, metadata)?;
    Ok(RustProject {
        name: config.name.clone(),
        manifest_path: config.manifest_path.clone(),
        root,
        packages,
        clippy_args: config.clippy_args.clone(),
        test_args: config.test_args.clone(),
    })
}

fn read_metadata(repo_root: &Path, config: &RustProjectConfig) -> Result<Metadata> {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(repo_root.join(&config.manifest_path))
        .current_dir(repo_root)
        .output()
        .with_context(|| format!("read Cargo metadata for {}", config.name))?;
    if !output.status.success() {
        bail!(
            "read Cargo metadata for {}: {}",
            config.name,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let metadata: Metadata = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("decode Cargo metadata for {}", config.name))?;
    Ok(metadata)
}

fn workspace_packages(repo_root: &Path, metadata: Metadata) -> Result<Vec<RustPackage>> {
    let workspace_members = metadata
        .workspace_members
        .into_iter()
        .collect::<BTreeSet<_>>();
    let metadata_packages = metadata
        .packages
        .into_iter()
        .filter(|package| workspace_members.contains(&package.id))
        .collect::<Vec<_>>();
    let names_by_root = metadata_packages
        .iter()
        .filter_map(|package| {
            package
                .manifest_path
                .parent()
                .map(|root| (root.to_path_buf(), package.name.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let packages = metadata_packages
        .into_iter()
        .map(|package| {
            let root = package
                .manifest_path
                .parent()
                .context("Cargo package manifest has no parent")?;
            let library_root = package
                .targets
                .iter()
                .find(|target| target.kind.iter().any(|kind| is_library_kind(kind)))
                .and_then(|target| target.src_path.parent())
                .map(|root| relative_to_repo(repo_root, root))
                .transpose()?;
            let non_library_entrypoints = package
                .targets
                .iter()
                .filter(|target| !target.kind.iter().any(|kind| is_library_kind(kind)))
                .map(|target| relative_to_repo(repo_root, &target.src_path))
                .collect::<Result<BTreeSet<_>>>()?;
            let dependencies = package
                .dependencies
                .iter()
                .filter_map(|dependency| dependency.path.as_ref())
                .filter_map(|path| names_by_root.get(path))
                .cloned()
                .collect();
            Ok(RustPackage {
                name: package.name,
                root: relative_to_repo(repo_root, root)?,
                library_root,
                non_library_entrypoints,
                dependencies,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if packages.is_empty() {
        bail!("Cargo project has no workspace packages");
    }
    Ok(packages)
}

fn is_library_kind(kind: &str) -> bool {
    kind == "lib" || kind.ends_with("lib") || kind == "proc-macro"
}

fn relative_to_repo(repo_root: &Path, path: &Path) -> Result<PathBuf> {
    path.strip_prefix(repo_root)
        .map(Path::to_path_buf)
        .map_err(|_| anyhow!("{} is outside {}", path.display(), repo_root.display()))
}

pub(crate) fn is_library_source(projects: &[RustProject], path: &Path) -> bool {
    let owner = projects
        .iter()
        .flat_map(|project| &project.packages)
        .filter(|package| path.starts_with(&package.root))
        .max_by_key(|package| package.root.components().count());
    let Some(owner) = owner else {
        return false;
    };
    if owner.non_library_entrypoints.contains(path) {
        return false;
    }
    owner
        .library_root
        .as_ref()
        .is_some_and(|root| path.starts_with(root))
}

pub(crate) fn scopes_for_staged_paths(
    projects: &[RustProject],
    staged_paths: &[PathBuf],
) -> BTreeMap<usize, CargoScope> {
    let mut scopes = BTreeMap::new();
    let config_changed = staged_paths
        .iter()
        .any(|path| path == Path::new(".githooks/config.toml"));

    for (index, project) in projects.iter().enumerate() {
        if config_changed {
            scopes.insert(index, CargoScope::Workspace);
            continue;
        }

        for path in staged_paths {
            add_path_scope(&mut scopes, index, project, path);
            if scopes.get(&index) == Some(&CargoScope::Workspace) {
                break;
            }
        }
        if let Some(CargoScope::Packages(packages)) = scopes.get_mut(&index) {
            add_reverse_dependencies(project, packages);
        }
    }
    scopes
}

fn add_reverse_dependencies(project: &RustProject, selected: &mut BTreeSet<String>) {
    loop {
        let dependents = project
            .packages
            .iter()
            .filter(|package| {
                package
                    .dependencies
                    .iter()
                    .any(|name| selected.contains(name))
            })
            .map(|package| package.name.clone())
            .collect::<BTreeSet<_>>();
        let previous_len = selected.len();
        selected.extend(dependents);
        if selected.len() == previous_len {
            return;
        }
    }
}

fn add_path_scope(
    scopes: &mut BTreeMap<usize, CargoScope>,
    index: usize,
    project: &RustProject,
    path: &Path,
) {
    if !path.starts_with(&project.root) {
        return;
    }

    let file_name = path.file_name().and_then(|name| name.to_str());
    if matches!(file_name, Some("Cargo.toml" | "Cargo.lock")) {
        scopes.insert(index, CargoScope::Workspace);
        return;
    }
    if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
        return;
    }
    if scopes.get(&index) == Some(&CargoScope::Workspace) {
        return;
    }

    let owner = project
        .packages
        .iter()
        .filter(|package| path.starts_with(&package.root))
        .max_by_key(|package| package.root.components().count());
    let Some(owner) = owner else {
        return;
    };

    match scopes
        .entry(index)
        .or_insert_with(|| CargoScope::Packages(BTreeSet::new()))
    {
        CargoScope::Packages(packages) => {
            packages.insert(owner.name.clone());
        }
        CargoScope::Workspace => {}
    }
}

#[cfg(test)]
#[path = "cargo_project_tests.rs"]
mod tests;
