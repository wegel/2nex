//! Manifest database deployment and dependency flattening for Nex capsules.

use std::fs;
use std::io;
use std::path::Path;

use walkdir::WalkDir;

use crate::manifest::ManifestIndex;
use crate::materializer::{flatten_capsule_precomputed, flatten_graphics_provider_files};

pub(super) fn deploy_manifests_to_nex_db(target_dir: &Path) -> io::Result<()> {
    let src_pkg_dir = Path::new("pkg");
    let dst_db_dir = target_dir.join("nex/db/pkg");

    if !src_pkg_dir.exists() {
        println!("  Warning: pkg/ directory not found, skipping manifest deployment");
        return Ok(());
    }

    println!("Deploying manifests to /nex/db/pkg...");
    fs::create_dir_all(&dst_db_dir)?;
    let count = copy_package_manifest_tree(src_pkg_dir, &dst_db_dir)?;
    println!("  Deployed {} manifest files to /nex/db/pkg", count);
    Ok(())
}

pub(super) fn flatten_package_dependencies(
    repo_path: &str,
    nex_pkg_dir: &Path,
    manifest_dir: &Path,
) -> io::Result<()> {
    println!("Flattening runtime dependencies into package capsules...");
    let manifest_index = ManifestIndex::load(manifest_dir)?;
    println!("  Loaded {} manifests", manifest_index.manifest_count());

    for package_dir in package_capsule_dirs(nex_pkg_dir) {
        flatten_package_capsule(repo_path, nex_pkg_dir, &manifest_index, &package_dir)?;
    }
    let graphics_count = flatten_graphics_provider_files(repo_path, nex_pkg_dir, &manifest_index)?;
    if graphics_count > 0 {
        println!(
            "  Flattened {} graphics provider files into capsules",
            graphics_count
        );
    }
    Ok(())
}

fn copy_package_manifest_tree(src_pkg_dir: &Path, dst_db_dir: &Path) -> io::Result<usize> {
    let mut count = 0;
    for entry in WalkDir::new(src_pkg_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let src_path = entry.path();
        let rel_path = src_path.strip_prefix(src_pkg_dir).unwrap_or(src_path);
        let dst_path = dst_db_dir.join(rel_path);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dst_path)?;
        } else if is_yaml_path(src_path) {
            copy_manifest_file(src_path, &dst_path)?;
            count += 1;
        }
    }
    Ok(count)
}

fn copy_manifest_file(src_path: &Path, dst_path: &Path) -> io::Result<()> {
    if let Some(parent) = dst_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src_path, dst_path)?;
    Ok(())
}

fn package_capsule_dirs(nex_pkg_dir: &Path) -> Vec<std::path::PathBuf> {
    WalkDir::new(nex_pkg_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_dir())
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| path.join(".nex-app-root").exists())
        .collect()
}

fn flatten_package_capsule(
    repo_path: &str,
    nex_pkg_dir: &Path,
    manifest_index: &ManifestIndex,
    pkg_dir: &Path,
) -> io::Result<()> {
    let commits = package_root_commits(pkg_dir)?;
    if commits.is_empty() {
        return Ok(());
    }
    let mut flattened_count = 0;
    for commit in commits {
        flattened_count +=
            flatten_capsule_precomputed(repo_path, pkg_dir, &commit, manifest_index, &[])?;
    }
    if flattened_count > 0 {
        let rel_path = pkg_dir.strip_prefix(nex_pkg_dir).unwrap_or(pkg_dir);
        println!(
            "  Flattened {} libs into {}",
            flattened_count,
            rel_path.display()
        );
    }
    Ok(())
}

fn package_root_commits(pkg_dir: &Path) -> io::Result<Vec<String>> {
    Ok(fs::read_to_string(pkg_dir.join(".nex-app-root"))?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect())
}

fn is_yaml_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == "yaml" || ext == "yml")
}

#[cfg(test)]
#[path = "nex_db_tests.rs"]
mod nex_db_tests;
