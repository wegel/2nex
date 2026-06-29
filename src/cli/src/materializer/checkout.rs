//! Materializer checkout modes and Nex symlink forest creation.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::ManifestIndex;

use super::checkout_refs::{get_package_id, PackageId};
use super::checkout_store::{checkout_commit_flat, checkout_files};
use super::flatten::flatten_capsule_precomputed;
use super::symlink_forest::create_symlink_forest_split;
use super::types::{MaterializeConfig, MaterializeMode, MaterializeResult, RuntimeClosure};

type PackageMap = BTreeMap<PackageId, Vec<String>>;
type RootCommitMap = BTreeMap<PackageId, Vec<String>>;
type PackageSet = HashSet<PackageId>;

struct NexPaths {
    physical_pkg: PathBuf,
    physical_env: PathBuf,
    logical_pkg: PathBuf,
    logical_env: PathBuf,
}

/// Checkout commits from the closure to the target directory.
pub fn checkout_closure(
    config: &MaterializeConfig,
    closure: &RuntimeClosure,
) -> io::Result<MaterializeResult> {
    let mut result = MaterializeResult::new(closure.clone());

    let manifest_index = if config.mode == MaterializeMode::Nex {
        if !config.manifest_db_paths.is_empty() {
            Some(ManifestIndex::load_layered(&config.manifest_db_paths)?)
        } else {
            None
        }
    } else {
        None
    };

    match config.mode {
        MaterializeMode::Flat => checkout_flat(config, closure, &mut result)?,
        MaterializeMode::Nex => {
            checkout_nex(config, closure, &mut result, manifest_index.as_ref())?
        }
    }

    Ok(result)
}

/// Flat checkout: union merge all commits into target directory.
fn checkout_flat(
    config: &MaterializeConfig,
    closure: &RuntimeClosure,
    _result: &mut MaterializeResult,
) -> io::Result<()> {
    fs::create_dir_all(&config.target_dir)?;

    for commit in closure.all_commits() {
        if let Some(files) = closure.get_files(commit) {
            let files_vec: Vec<String> = files.iter().cloned().collect();
            println!("  Checking out {} file(s) from {}", files_vec.len(), commit);
            checkout_files(
                &config.repo_path,
                commit,
                &files_vec,
                &config.target_dir,
                &config.fallback_repo_paths,
            )?;
        } else {
            println!("  Checking out {} (flat)", commit);
            checkout_commit_flat(
                &config.repo_path,
                commit,
                &config.target_dir,
                &config.fallback_repo_paths,
            )?;
        }
    }

    Ok(())
}

fn checkout_nex(
    config: &MaterializeConfig,
    closure: &RuntimeClosure,
    result: &mut MaterializeResult,
    manifest_index: Option<&ManifestIndex>,
) -> io::Result<()> {
    let paths = nex_paths(config)?;
    let (packages, root_commits) = package_groups(config, closure)?;
    let root_packages = root_package_set(&root_commits);
    checkout_root_packages(config, &paths, &packages, &root_commits)?;

    if let Some(idx) = manifest_index {
        flatten_all_capsules_split(
            &config.repo_path,
            &paths.physical_pkg,
            &paths.logical_pkg,
            &root_commits,
            idx,
            &config.fallback_repo_paths,
        )?;
    }

    create_symlink_forest_split(
        &paths.physical_pkg,
        &paths.logical_pkg,
        &paths.physical_env,
        &paths.logical_env,
        &packages,
        &root_packages,
        result,
    )?;

    Ok(())
}

fn nex_paths(config: &MaterializeConfig) -> io::Result<NexPaths> {
    let physical_root = config.physical_root.as_ref().unwrap_or(&config.target_dir);
    let physical_pkg = config
        .pkg_dir_override
        .clone()
        .unwrap_or_else(|| physical_root.join("nex/pkg"));
    let physical_env = config
        .env_dir_override
        .clone()
        .unwrap_or_else(|| physical_root.join("nex/env"));
    let logical_pkg = config
        .pkg_dir_override
        .clone()
        .unwrap_or_else(|| config.target_dir.join("nex/pkg"));
    let logical_env = config
        .env_dir_override
        .clone()
        .unwrap_or_else(|| config.target_dir.join("nex/env"));
    fs::create_dir_all(&physical_pkg)?;
    fs::create_dir_all(&physical_env)?;
    Ok(NexPaths {
        physical_pkg,
        physical_env,
        logical_pkg,
        logical_env,
    })
}

fn package_groups(
    config: &MaterializeConfig,
    closure: &RuntimeClosure,
) -> io::Result<(PackageMap, RootCommitMap)> {
    let mut packages: PackageMap = BTreeMap::new();
    let mut root_commits: RootCommitMap = BTreeMap::new();

    for commit in closure.all_commits() {
        let pkg_id = get_package_id(&config.repo_path, commit, &config.fallback_repo_paths)?;
        packages
            .entry(pkg_id.clone())
            .or_default()
            .push(commit.clone());
        if closure.is_root(commit) {
            root_commits.entry(pkg_id).or_default().push(commit.clone());
        }
    }

    Ok((packages, root_commits))
}

fn root_package_set(root_commits: &RootCommitMap) -> PackageSet {
    root_commits.keys().cloned().collect()
}

fn checkout_root_packages(
    config: &MaterializeConfig,
    paths: &NexPaths,
    packages: &PackageMap,
    root_commits: &RootCommitMap,
) -> io::Result<()> {
    for (pkg_id, commits) in packages {
        if let Some(root_commits) = root_commits.get(pkg_id) {
            checkout_root_package(config, paths, pkg_id, commits, root_commits)?;
        }
    }
    Ok(())
}

fn checkout_root_package(
    config: &MaterializeConfig,
    paths: &NexPaths,
    pkg_id: &PackageId,
    commits: &[String],
    root_commits: &[String],
) -> io::Result<()> {
    let short_hash = &pkg_id.manifest_hash[..8.min(pkg_id.manifest_hash.len())];
    let physical_pkg_dir = paths.physical_pkg.join(&pkg_id.path).join(short_hash);
    let logical_pkg_dir = paths.logical_pkg.join(&pkg_id.path).join(short_hash);
    if logical_pkg_dir.exists() {
        println!("  Merging new output(s) into {}", pkg_id.path);
    }

    print_checkout_package(pkg_id, short_hash, commits.len());
    fs::create_dir_all(&physical_pkg_dir)?;
    for commit in commits {
        checkout_commit_flat(
            &config.repo_path,
            commit,
            &physical_pkg_dir,
            &config.fallback_repo_paths,
        )?;
    }
    write_root_commits(&physical_pkg_dir, root_commits)
}

fn write_root_commits(pkg_dir: &Path, root_commits: &[String]) -> io::Result<()> {
    let marker = pkg_dir.join(".nex-app-root");
    let mut content = if marker.exists() {
        fs::read_to_string(&marker)?
    } else {
        String::new()
    };
    let mut seen = content.lines().map(str::to_string).collect::<HashSet<_>>();
    for commit in root_commits {
        if seen.insert(commit.clone()) {
            content.push_str(commit);
            content.push('\n');
        }
    }
    fs::write(marker, content)
}

fn print_checkout_package(pkg_id: &PackageId, short_hash: &str, commit_count: usize) {
    println!(
        "  Checking out {} ({} output{}) -> /nex/pkg/{}/{}",
        pkg_id.path,
        commit_count,
        if commit_count > 1 { "s" } else { "" },
        pkg_id.path,
        short_hash
    );
}

fn flatten_all_capsules_split(
    repo_path: &str,
    physical_nex_pkg: &Path,
    logical_nex_pkg: &Path,
    root_commits: &RootCommitMap,
    manifest_index: &ManifestIndex,
    fallback_repos: &[PathBuf],
) -> io::Result<()> {
    for (pkg_id, commits) in root_commits {
        let short_hash = &pkg_id.manifest_hash[..8.min(pkg_id.manifest_hash.len())];
        let physical_pkg_dir = physical_nex_pkg.join(&pkg_id.path).join(short_hash);
        let logical_pkg_dir = logical_nex_pkg.join(&pkg_id.path).join(short_hash);

        if !logical_pkg_dir.exists() && !physical_pkg_dir.exists() {
            continue;
        }

        for commit in commits {
            let flattened_count = flatten_capsule_precomputed(
                repo_path,
                &physical_pkg_dir,
                commit,
                manifest_index,
                fallback_repos,
            )?;
            if flattened_count > 0 {
                println!(
                    "  Flattened {} libs into {}/{}",
                    flattened_count, pkg_id.path, short_hash
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "checkout_tests.rs"]
mod checkout_tests;
