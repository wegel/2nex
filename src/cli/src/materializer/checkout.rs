//! Materializer checkout modes and Nex symlink forest creation.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::ManifestIndex;

use super::checkout_refs::{get_package_id, PackageId};
use super::checkout_store::{checkout_commit_flat, checkout_files};
use super::flatten::flatten_capsule_precomputed;
use super::pathdiff::diff_paths;
use super::types::{MaterializeConfig, MaterializeMode, MaterializeResult, RuntimeClosure};

type PackageMap = BTreeMap<PackageId, Vec<String>>;
type PackageSet = HashSet<PackageId>;

struct NexPaths {
    physical_pkg: PathBuf,
    physical_env: PathBuf,
    logical_pkg: PathBuf,
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

/// Nex checkout: isolated package directories with symlink forests.
/// Groups outputs by manifest hash so relative symlinks within a package work.
///
/// When `physical_root` is set (staging mode), files are written there and overlays
/// make them visible at the logical paths.
fn checkout_nex(
    config: &MaterializeConfig,
    closure: &RuntimeClosure,
    result: &mut MaterializeResult,
    manifest_index: Option<&ManifestIndex>,
) -> io::Result<()> {
    let paths = nex_paths(config)?;
    let (packages, root_packages) = package_groups(config, closure)?;
    checkout_root_packages(config, &paths, &packages, &root_packages)?;

    if let Some(idx) = manifest_index {
        flatten_all_capsules_split(
            &config.repo_path,
            &paths.physical_pkg,
            &paths.logical_pkg,
            &packages,
            &root_packages,
            idx,
            &config.fallback_repo_paths,
        )?;
    }

    create_symlink_forest_split(
        config,
        &paths.physical_pkg,
        &paths.physical_env,
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
    fs::create_dir_all(&physical_pkg)?;
    fs::create_dir_all(&physical_env)?;
    Ok(NexPaths {
        physical_pkg,
        physical_env,
        logical_pkg,
    })
}

fn package_groups(
    config: &MaterializeConfig,
    closure: &RuntimeClosure,
) -> io::Result<(PackageMap, PackageSet)> {
    let mut packages: PackageMap = BTreeMap::new();
    let mut root_packages = HashSet::new();

    for commit in closure.all_commits() {
        let pkg_id = get_package_id(&config.repo_path, commit, &config.fallback_repo_paths)?;
        packages
            .entry(pkg_id.clone())
            .or_default()
            .push(commit.clone());
        if closure.is_root(commit) {
            root_packages.insert(pkg_id);
        }
    }

    Ok((packages, root_packages))
}

fn checkout_root_packages(
    config: &MaterializeConfig,
    paths: &NexPaths,
    packages: &PackageMap,
    root_packages: &PackageSet,
) -> io::Result<()> {
    for (pkg_id, commits) in packages {
        if root_packages.contains(pkg_id) {
            checkout_root_package(config, paths, pkg_id, commits)?;
        }
    }
    Ok(())
}

fn checkout_root_package(
    config: &MaterializeConfig,
    paths: &NexPaths,
    pkg_id: &PackageId,
    commits: &[String],
) -> io::Result<()> {
    let short_hash = &pkg_id.manifest_hash[..8.min(pkg_id.manifest_hash.len())];
    let physical_pkg_dir = paths.physical_pkg.join(&pkg_id.path).join(short_hash);
    let logical_pkg_dir = paths.logical_pkg.join(&pkg_id.path).join(short_hash);
    if logical_pkg_dir.exists() {
        println!("  {} already materialized", pkg_id.path);
        return Ok(());
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
    fs::write(
        physical_pkg_dir.join(".nex-app-root"),
        commits.join("\n") + "\n",
    )
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

/// Flatten runtime dependencies into root package capsules using precomputed deps.
/// Uses physical path for writing, logical path for checking existing.
fn flatten_all_capsules_split(
    repo_path: &str,
    physical_nex_pkg: &Path,
    logical_nex_pkg: &Path,
    packages: &PackageMap,
    root_packages: &PackageSet,
    manifest_index: &ManifestIndex,
    fallback_repos: &[PathBuf],
) -> io::Result<()> {
    for (pkg_id, commits) in packages {
        if !root_packages.contains(pkg_id) {
            continue;
        }

        let short_hash = &pkg_id.manifest_hash[..8.min(pkg_id.manifest_hash.len())];
        let physical_pkg_dir = physical_nex_pkg.join(&pkg_id.path).join(short_hash);
        let logical_pkg_dir = logical_nex_pkg.join(&pkg_id.path).join(short_hash);

        if !logical_pkg_dir.exists() && !physical_pkg_dir.exists() {
            continue;
        }

        if let Some(commit) = commits.first() {
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

/// Create symlink forest in /nex/env pointing to package files.
/// Writes symlinks to physical paths but targets are relative to logical root.
fn create_symlink_forest_split(
    config: &MaterializeConfig,
    physical_nex_pkg: &Path,
    physical_nex_env: &Path,
    packages: &PackageMap,
    root_packages: &PackageSet,
    result: &mut MaterializeResult,
) -> io::Result<()> {
    let logical_nex_pkg = config.target_dir.join("nex/pkg");
    let logical_nex_env = config.target_dir.join("nex/env");
    let link_dirs = ["bin", "lib", "lib64", "sbin", "share", "include"];

    for pkg_id in packages.keys() {
        if let Some(pkg_dir) =
            symlink_source_package_dir(pkg_id, root_packages, physical_nex_pkg, &logical_nex_pkg)
        {
            create_package_symlinks(
                &pkg_dir,
                physical_nex_env,
                &logical_nex_env,
                &link_dirs,
                result,
            )?;
        }
    }

    Ok(())
}

fn symlink_source_package_dir(
    pkg_id: &PackageId,
    root_packages: &PackageSet,
    physical_nex_pkg: &Path,
    logical_nex_pkg: &Path,
) -> Option<PathBuf> {
    if !root_packages.contains(pkg_id) {
        return None;
    }
    let short_hash = &pkg_id.manifest_hash[..8.min(pkg_id.manifest_hash.len())];
    let physical_pkg_dir = physical_nex_pkg.join(&pkg_id.path).join(short_hash);
    let logical_pkg_dir = logical_nex_pkg.join(&pkg_id.path).join(short_hash);

    if logical_pkg_dir.exists() {
        Some(logical_pkg_dir)
    } else if physical_pkg_dir.exists() {
        Some(physical_pkg_dir)
    } else {
        None
    }
}

fn create_package_symlinks(
    pkg_dir: &Path,
    physical_nex_env: &Path,
    logical_nex_env: &Path,
    link_dirs: &[&str],
    result: &mut MaterializeResult,
) -> io::Result<()> {
    for subdir in link_dirs {
        let src_paths = [pkg_dir.join("usr").join(subdir), pkg_dir.join(subdir)];
        for src in &src_paths {
            if src.exists() {
                let physical_env_subdir = physical_nex_env.join(subdir);
                let logical_env_subdir = logical_nex_env.join(subdir);
                fs::create_dir_all(&physical_env_subdir)?;
                create_symlinks_split(src, &physical_env_subdir, &logical_env_subdir, result)?;
            }
        }
    }
    Ok(())
}

/// Recursively create symlinks from source to target directory (split physical/logical).
fn create_symlinks_split(
    src: &Path,
    physical_env_dir: &Path,
    logical_env_dir: &Path,
    result: &mut MaterializeResult,
) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let physical_target = physical_env_dir.join(&name);
        let logical_target = logical_env_dir.join(&name);

        if path.is_dir() {
            fs::create_dir_all(&physical_target)?;
            create_symlinks_split(&path, &physical_target, &logical_target, result)?;
        } else {
            if logical_target.exists() || logical_target.symlink_metadata().is_ok() {
                result.warn(format!(
                    "Skipping {}: already exists",
                    logical_target.display()
                ));
                continue;
            }

            if physical_target.exists() || physical_target.symlink_metadata().is_ok() {
                result.warn(format!(
                    "Skipping {}: already exists in staging",
                    physical_target.display()
                ));
                continue;
            }

            let relative_target =
                diff_paths(&path, logical_env_dir).unwrap_or_else(|| path.clone());

            std::os::unix::fs::symlink(&relative_target, &physical_target)?;
            result.symlinks_created.push(logical_target);
        }
    }

    Ok(())
}
