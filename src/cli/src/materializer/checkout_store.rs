//! Store checkout helpers for flat and file-level materialization.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::store::Store;

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
    if !src.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File {} not found in commit {}", file, commit),
        ));
    }

    let dst = target_dir.join(file.trim_start_matches('/'));
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    if src.symlink_metadata()?.file_type().is_symlink() {
        checkout_symlink_file(&src, &dst)
    } else {
        copy_regular_file(&src, &dst)
    }
}

fn checkout_symlink_file(src: &Path, dst: &Path) -> io::Result<()> {
    let link_target = fs::read_link(src)?;
    if !link_target.is_absolute() {
        copy_relative_symlink_target(src, dst, &link_target)?;
    }
    if dst.exists() || dst.symlink_metadata().is_ok() {
        fs::remove_file(dst)?;
    }
    std::os::unix::fs::symlink(link_target, dst)
}

fn copy_relative_symlink_target(src: &Path, dst: &Path, link_target: &Path) -> io::Result<()> {
    let target_src = src.parent().unwrap().join(link_target);
    if !target_src.exists() || target_src.symlink_metadata()?.file_type().is_symlink() {
        return Ok(());
    }

    let target_dst = dst.parent().unwrap().join(link_target);
    if !target_dst.exists() {
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
