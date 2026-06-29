//! Nex environment symlink forest creation.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::checkout_refs::PackageId;
use super::pathdiff::diff_paths;
use super::types::MaterializeResult;

struct SymlinkSource {
    physical: PathBuf,
    logical: PathBuf,
}

pub(super) fn create_symlink_forest_split(
    physical_nex_pkg: &Path,
    logical_nex_pkg: &Path,
    physical_nex_env: &Path,
    logical_nex_env: &Path,
    packages: &BTreeMap<PackageId, Vec<String>>,
    root_packages: &HashSet<PackageId>,
    result: &mut MaterializeResult,
) -> io::Result<()> {
    let link_dirs = ["bin", "lib", "lib64", "sbin", "share", "include"];

    for pkg_id in packages.keys() {
        if let Some(pkg_dir) =
            symlink_source_package_dir(pkg_id, root_packages, physical_nex_pkg, logical_nex_pkg)
        {
            create_package_symlinks(
                &pkg_dir,
                physical_nex_env,
                logical_nex_env,
                &link_dirs,
                result,
            )?;
        }
    }

    Ok(())
}

fn symlink_source_package_dir(
    pkg_id: &PackageId,
    root_packages: &HashSet<PackageId>,
    physical_nex_pkg: &Path,
    logical_nex_pkg: &Path,
) -> Option<SymlinkSource> {
    if !root_packages.contains(pkg_id) {
        return None;
    }
    let short_hash = &pkg_id.manifest_hash[..8.min(pkg_id.manifest_hash.len())];
    let physical_pkg_dir = physical_nex_pkg.join(&pkg_id.path).join(short_hash);
    let logical_pkg_dir = logical_nex_pkg.join(&pkg_id.path).join(short_hash);

    if physical_pkg_dir.exists() || logical_pkg_dir.exists() {
        Some(SymlinkSource {
            physical: if physical_pkg_dir.exists() {
                physical_pkg_dir
            } else {
                logical_pkg_dir.clone()
            },
            logical: logical_pkg_dir,
        })
    } else {
        None
    }
}

fn create_package_symlinks(
    pkg_dir: &SymlinkSource,
    physical_nex_env: &Path,
    logical_nex_env: &Path,
    link_dirs: &[&str],
    result: &mut MaterializeResult,
) -> io::Result<()> {
    for subdir in link_dirs {
        let src_paths = [
            (
                pkg_dir.physical.join("usr").join(subdir),
                pkg_dir.logical.join("usr").join(subdir),
            ),
            (pkg_dir.physical.join(subdir), pkg_dir.logical.join(subdir)),
        ];
        for (physical_src, logical_src) in &src_paths {
            if physical_src.exists() {
                let physical_env_subdir = physical_nex_env.join(subdir);
                let logical_env_subdir = logical_nex_env.join(subdir);
                fs::create_dir_all(&physical_env_subdir)?;
                create_symlinks_split(
                    physical_src,
                    logical_src,
                    &physical_env_subdir,
                    &logical_env_subdir,
                    result,
                )?;
            }
        }
    }
    Ok(())
}

fn create_symlinks_split(
    physical_src: &Path,
    logical_src: &Path,
    physical_env_dir: &Path,
    logical_env_dir: &Path,
    result: &mut MaterializeResult,
) -> io::Result<()> {
    for entry in fs::read_dir(physical_src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let logical_path = logical_src.join(&name);
        let physical_target = physical_env_dir.join(&name);
        let logical_target = logical_env_dir.join(&name);

        if path.is_dir() {
            fs::create_dir_all(&physical_target)?;
            create_symlinks_split(
                &path,
                &logical_path,
                &physical_target,
                &logical_target,
                result,
            )?;
        } else {
            create_symlink_entry(&logical_path, &physical_target, &logical_target, result)?;
        }
    }

    Ok(())
}

fn create_symlink_entry(
    logical_path: &Path,
    physical_target: &Path,
    logical_target: &Path,
    result: &mut MaterializeResult,
) -> io::Result<()> {
    if logical_target.exists() || logical_target.symlink_metadata().is_ok() {
        result.warn(format!(
            "Skipping {}: already exists",
            logical_target.display()
        ));
        return Ok(());
    }

    if physical_target.exists() || physical_target.symlink_metadata().is_ok() {
        result.warn(format!(
            "Skipping {}: already exists in staging",
            physical_target.display()
        ));
        return Ok(());
    }

    let logical_env_dir = logical_target.parent().unwrap_or_else(|| Path::new(""));
    let relative_target =
        diff_paths(logical_path, logical_env_dir).unwrap_or_else(|| logical_path.to_path_buf());

    std::os::unix::fs::symlink(&relative_target, physical_target)?;
    result.symlinks_created.push(logical_target.to_path_buf());
    Ok(())
}
