//! Store checkout helpers for flat and file-level materialization.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::store::Store;

use super::relative_symlink::{
    reject_too_many_symlink_hops, relative_symlink_target_inside_root, MAX_RELATIVE_SYMLINK_HOPS,
};

/// Checkout a single commit in flat mode with union semantics.
pub(super) fn checkout_commit_flat(
    repo_path: &str,
    commit: &str,
    target_dir: &Path,
    fallback_repos: &[PathBuf],
) -> io::Result<()> {
    let store = Store::open_with_fallback_chain(repo_path, fallback_repos)?;
    store.checkout(commit, target_dir, true)
}

/// Checkout specific files from a commit to a target directory.
pub fn checkout_files(
    repo_path: &str,
    commit: &str,
    files: &[String],
    target_dir: &Path,
    fallback_repos: &[PathBuf],
) -> io::Result<()> {
    let temp = TempDir::new_in(target_dir)?;
    checkout_commit_flat(repo_path, commit, temp.path(), fallback_repos)?;

    for file in files {
        checkout_one_file(temp.path(), file, target_dir, commit)?;
    }

    Ok(())
}

fn checkout_one_file(
    temp_dir: &Path,
    file: &str,
    target_dir: &Path,
    commit: &str,
) -> io::Result<()> {
    let src = temp_dir.join(file.trim_start_matches('/'));
    let src_metadata = src.symlink_metadata().map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("File {} not found in commit {}", file, commit),
            )
        } else {
            error
        }
    })?;

    let dst = target_dir.join(file.trim_start_matches('/'));
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    if src_metadata.file_type().is_symlink() {
        checkout_symlink_file(temp_dir, target_dir, &src, &dst)
    } else {
        copy_regular_file(&src, &dst)
    }
}

fn checkout_symlink_file(
    checkout_root: &Path,
    target_root: &Path,
    src: &Path,
    dst: &Path,
) -> io::Result<()> {
    let link_target = fs::read_link(src)?;
    if !link_target.is_absolute() {
        copy_relative_symlink_target(checkout_root, target_root, src, &link_target, 0)?;
    }
    if dst.exists() || dst.symlink_metadata().is_ok() {
        fs::remove_file(dst)?;
    }
    std::os::unix::fs::symlink(link_target, dst)
}

fn copy_relative_symlink_target(
    checkout_root: &Path,
    target_root: &Path,
    src: &Path,
    link_target: &Path,
    depth: usize,
) -> io::Result<()> {
    if depth > MAX_RELATIVE_SYMLINK_HOPS {
        return reject_too_many_symlink_hops(src);
    }

    let relative_target = relative_symlink_target_inside_root(checkout_root, src, link_target)?;
    let target_src = checkout_root.join(&relative_target);
    if !target_src.exists() && target_src.symlink_metadata().is_err() {
        return Ok(());
    }

    let target_dst = target_root.join(relative_target);
    if let Some(parent) = target_dst.parent() {
        fs::create_dir_all(parent)?;
    }

    let metadata = target_src.symlink_metadata()?;
    if metadata.file_type().is_symlink() {
        let next_target = fs::read_link(&target_src)?;
        if target_dst.exists() || target_dst.symlink_metadata().is_ok() {
            fs::remove_file(&target_dst)?;
        }
        std::os::unix::fs::symlink(&next_target, &target_dst)?;
        if !next_target.is_absolute() {
            copy_relative_symlink_target(
                checkout_root,
                target_root,
                &target_src,
                &next_target,
                depth + 1,
            )?;
        }
    } else if metadata.is_dir() {
        fs::create_dir_all(&target_dst)?;
    } else if !target_dst.exists() {
        fs::copy(target_src, target_dst)?;
    }
    Ok(())
}

fn copy_regular_file(src: &Path, dst: &Path) -> io::Result<()> {
    if is_same_inode(src, dst) {
        return Ok(());
    }
    fs::copy(src, dst)?;
    Ok(())
}

fn is_same_inode(src: &Path, dst: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    let src_ino = fs::metadata(src).map(|metadata| metadata.ino()).ok();
    let dst_ino = if dst.exists() {
        fs::metadata(dst).map(|metadata| metadata.ino()).ok()
    } else {
        None
    };
    src_ino.is_some() && src_ino == dst_ino
}

#[cfg(test)]
#[path = "checkout_store_tests.rs"]
mod checkout_store_tests;
