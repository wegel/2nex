//! Symlink helpers for Nex package capsules and FHS compatibility paths.

use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::nex::{package_symlink_target, PackageInstall};

pub(super) fn symlink_flattened_libs_to_usr(
    nex_pkg_dir: &Path,
    target_dir: &Path,
) -> io::Result<()> {
    let usr_lib = target_dir.join("usr/lib");
    fs::create_dir_all(&usr_lib)?;

    for capsule_usr_lib in sorted_capsule_usr_lib_dirs(nex_pkg_dir) {
        symlink_usr_lib_entries(nex_pkg_dir, &usr_lib, &capsule_usr_lib)?;
    }

    Ok(())
}

/// Collect every capsule `usr/lib` directory in path order. The first capsule
/// that provides a library name wins, so the walk must not let filesystem
/// readdir order pick the provider. Ext4 and tmpfs return different orders for
/// the same tree, which made one assembly build to two different checksums.
fn sorted_capsule_usr_lib_dirs(nex_pkg_dir: &Path) -> Vec<PathBuf> {
    let mut capsule_dirs: Vec<PathBuf> = WalkDir::new(nex_pkg_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| is_capsule_usr_lib(entry.path()))
        .map(|entry| entry.path().to_path_buf())
        .collect();
    capsule_dirs.sort();
    capsule_dirs
}

pub(super) fn create_file_symlinks_recursive(
    src_dir: &Path,
    dst_dir: &Path,
    install: &PackageInstall,
    relative_base: &str,
) -> io::Result<()> {
    for entry in WalkDir::new(src_dir).min_depth(1).sort_by_file_name() {
        let entry = entry?;
        let rel_path = entry
            .path()
            .strip_prefix(src_dir)
            .expect("walked entry should stay under source directory");
        let dst_path = dst_dir.join(rel_path);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dst_path)?;
        } else if dst_path.symlink_metadata().is_err() {
            symlink(
                package_symlink_target(install, relative_base, rel_path),
                &dst_path,
            )?;
        }
    }
    Ok(())
}

pub(super) fn create_target_fhs_symlinks(target_dir: &Path) -> io::Result<()> {
    for (link_name, target) in [("bin", "usr/bin"), ("sbin", "usr/bin"), ("lib", "usr/lib")] {
        let link_path = target_dir.join(link_name);
        if !link_path.exists() {
            symlink(target, &link_path)?;
        }
    }

    let usr_lib = target_dir.join("usr/lib");
    if !usr_lib.exists() {
        fs::create_dir_all(&usr_lib)?;
    }

    let usr_sbin = target_dir.join("usr/sbin");
    if !usr_sbin.exists() {
        symlink("bin", &usr_sbin)?;
    }

    Ok(())
}

fn is_capsule_usr_lib(path: &Path) -> bool {
    path.is_dir() && path.to_string_lossy().ends_with("/usr/lib")
}

fn symlink_usr_lib_entries(
    nex_pkg_dir: &Path,
    usr_lib: &Path,
    capsule_usr_lib: &Path,
) -> io::Result<()> {
    let mut lib_paths: Vec<PathBuf> = WalkDir::new(capsule_usr_lib)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .map(|lib_entry| lib_entry.path().to_path_buf())
        .collect();
    lib_paths.sort();
    for lib_path in &lib_paths {
        symlink_usr_lib_entry(nex_pkg_dir, usr_lib, lib_path)?;
    }
    Ok(())
}

fn symlink_usr_lib_entry(nex_pkg_dir: &Path, usr_lib: &Path, lib_path: &Path) -> io::Result<()> {
    let Some(lib_name) = lib_path.file_name() else {
        return Ok(());
    };
    if is_dynamic_loader_name(lib_name.to_string_lossy().as_ref()) {
        return Ok(());
    }

    let target_path = usr_lib.join(lib_name);
    if target_path.exists() || target_path.symlink_metadata().is_ok() {
        return Ok(());
    }
    if let Ok(rel_from_nex_pkg) = lib_path.strip_prefix(nex_pkg_dir) {
        let symlink_target = Path::new("../../nex/pkg").join(rel_from_nex_pkg);
        let _ = symlink(&symlink_target, &target_path);
    }
    Ok(())
}

fn is_dynamic_loader_name(lib_name: &str) -> bool {
    lib_name.starts_with("ld-linux") || lib_name == "ld.so"
}

#[cfg(test)]
#[path = "nex_links_tests.rs"]
mod nex_links_tests;
