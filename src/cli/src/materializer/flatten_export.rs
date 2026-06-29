//! Direct file export helpers for capsule flattening.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::flatten_errors::flatten_export_error;
use super::relative_symlink::{
    reject_too_many_symlink_hops, relative_symlink_target_inside_root, MAX_RELATIVE_SYMLINK_HOPS,
};

/// Flatten a single library from a store commit, preserving original path structure.
pub(super) fn flatten_library_preserving_path(
    repo_path: &str,
    commit: &str,
    lib_path: &str,
    pkg_dir: &Path,
    fallback_repos: &[PathBuf],
) -> io::Result<bool> {
    let rel_path = lib_path.trim_start_matches('/');
    let dest = pkg_dir.join(rel_path);

    if dest.exists() {
        return Ok(false);
    }

    export_single_file(repo_path, commit, lib_path, &dest, fallback_repos)
        .map_err(|error| flatten_export_error(commit, lib_path, error))?;

    export_relative_symlink_targets(repo_path, commit, pkg_dir, &dest, fallback_repos, 0)?;

    Ok(true)
}

fn export_relative_symlink_targets(
    repo_path: &str,
    commit: &str,
    pkg_dir: &Path,
    src: &Path,
    fallback_repos: &[PathBuf],
    depth: usize,
) -> io::Result<()> {
    if depth > MAX_RELATIVE_SYMLINK_HOPS {
        return reject_too_many_symlink_hops(src);
    }

    let metadata = match src.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if !metadata.file_type().is_symlink() {
        return Ok(());
    }

    let link_target = fs::read_link(src)?;
    if link_target.is_absolute() {
        return Ok(());
    }

    let target_rel = relative_symlink_target_inside_root(pkg_dir, src, &link_target)?;
    let target_dest = pkg_dir.join(&target_rel);
    if !target_dest.exists() && target_dest.symlink_metadata().is_err() {
        let target_src_path = format!("/{}", target_rel.display());
        export_single_file(
            repo_path,
            commit,
            &target_src_path,
            &target_dest,
            fallback_repos,
        )
        .map_err(|error| flatten_export_error(commit, &target_src_path, error))?;
    }

    export_relative_symlink_targets(
        repo_path,
        commit,
        pkg_dir,
        &target_dest,
        fallback_repos,
        depth + 1,
    )
}

/// Export a single file from a commit using zub's export_path.
fn export_single_file(
    repo_path: &str,
    commit: &str,
    src_path: &str,
    dest: &Path,
    fallback_repos: &[PathBuf],
) -> io::Result<()> {
    use zub::Repo;

    let opts = zub::ops::ExportOptions {
        overwrite: true,
        hardlink: true,
        preserve_sparse: false,
    };

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    match zub::ops::export_path(&repo, commit, src_path, dest, opts.clone()) {
        Ok(()) => return Ok(()),
        Err(zub::Error::RefNotFound(_)) | Err(zub::Error::PathNotFound(_)) => {}
        Err(e) => return Err(io::Error::other(e.to_string())),
    }

    for fallback_path in fallback_repos {
        if let Ok(fallback) = Repo::open(fallback_path) {
            match zub::ops::export_path(&fallback, commit, src_path, dest, opts.clone()) {
                Ok(()) => return Ok(()),
                Err(zub::Error::RefNotFound(_)) | Err(zub::Error::PathNotFound(_)) => continue,
                Err(e) => return Err(io::Error::other(e.to_string())),
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("path {} not found in commit {}", src_path, commit),
    ))
}
