//! Build root filesystem setup and dependency layering.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::types::BuildPaths;
use crate::materializer::{materialize, MaterializeConfig, MaterializeMode, MaterializeRequest};
use crate::store::{checkout_into, checkout_into_with_fallbacks};

/// Inputs that describe how to prepare a composite build root.
pub struct RootfsSetup<'a> {
    /// Build root directory to create or reuse.
    pub base_dir: &'a str,
    /// Primary zub repository path.
    pub repo_path: &'a str,
    /// Fallback zub repositories used when a ref is not in the primary store.
    pub fallback_repos: &'a [String],
    /// Dependency commits to layer or hydrate.
    pub dependency_commits: &'a [String],
    /// Package manifest databases used to hydrate runtime dependencies.
    pub manifest_dirs: &'a [PathBuf],
    /// Build environment paths used to find the output directory.
    pub paths: &'a BuildPaths,
    /// Print verbose checkout output.
    pub verbose: bool,
    /// Reuse an existing rootfs when it is already present.
    pub reuse_rootfs: bool,
    /// Hydrate runtime dependencies for chroot builds before layering packages.
    pub hydrate_runtime_deps: bool,
}

/// Checkout existing package outputs into a new build output directory.
pub fn stage_existing_outputs(
    manifest: &crate::manifest::types::Manifest,
    base_dir: &str,
    repo_path: &str,
    paths: &BuildPaths,
) -> io::Result<()> {
    let base_path = Path::new(base_dir);
    if base_path.exists() {
        fs::remove_dir_all(base_path)?;
    }
    let out_dir = base_path.join(&paths.out);
    fs::create_dir_all(&out_dir)?;

    for category in manifest.outputs.keys() {
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.namespace_path(),
            manifest.package.slug,
            manifest.package.version,
            category
        );
        println!(
            "Checking out existing outputs from {} into {}",
            branch_name,
            out_dir.display()
        );
        checkout_into(repo_path, &branch_name, &out_dir, true)?;
    }

    Ok(())
}

/// Prepare a build root and layer build dependency commits into it.
pub fn setup_composite_rootfs(setup: RootfsSetup<'_>) -> io::Result<()> {
    let RootfsSetup {
        base_dir,
        repo_path,
        fallback_repos,
        dependency_commits,
        manifest_dirs,
        paths,
        verbose,
        reuse_rootfs,
        hydrate_runtime_deps,
    } = setup;

    println!("Setting up composite rootfs at {}", base_dir);
    reset_build_root(base_dir, paths, verbose, reuse_rootfs)?;

    if hydrate_runtime_deps && !dependency_commits.is_empty() {
        materialize_build_dependencies(
            base_dir,
            repo_path,
            fallback_repos,
            dependency_commits,
            manifest_dirs,
        )?;
    } else {
        layer_commits_into_rootfs(
            base_dir,
            repo_path,
            fallback_repos,
            dependency_commits,
            verbose,
        )?;
    }

    if !dependency_commits.is_empty() {
        create_usrmerge_symlinks(Path::new(base_dir))?;
    }

    Ok(())
}

/// Checkout commits into an existing build root.
pub fn layer_commits_into_rootfs(
    base_dir: &str,
    repo_path: &str,
    fallback_repos: &[String],
    commits: &[String],
    verbose: bool,
) -> io::Result<()> {
    for commit in commits {
        checkout_into_with_fallbacks(
            repo_path,
            fallback_repos,
            commit,
            Path::new(base_dir),
            true,
            verbose,
        )?;
    }
    Ok(())
}

pub(crate) fn materialize_request_for_build_dependency(commit: String) -> MaterializeRequest {
    if commit.contains("/bundles/") {
        MaterializeRequest::Bundle { commit }
    } else {
        MaterializeRequest::Output { commit }
    }
}

pub(crate) fn refresh_chroot_usrmerge_symlinks(build_dir: &Path) -> io::Result<()> {
    let mapped_uid = nix::unistd::Uid::effective().as_raw();
    let mapped_gid = nix::unistd::Gid::effective().as_raw();

    for dir in [build_dir.to_path_buf(), build_dir.join("usr")] {
        if dir.exists() {
            std::os::unix::fs::chown(&dir, Some(mapped_uid), Some(mapped_gid))?;
        }
    }

    remove_existing_usrmerge_symlinks(build_dir)?;
    create_owned_usrmerge_symlinks(build_dir, mapped_uid, mapped_gid)?;
    Ok(())
}

pub(crate) fn validate_chroot_build_root(build_dir: &Path) -> io::Result<()> {
    let bash = build_dir.join("usr/bin/bash");
    if !bash.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "chroot build root is missing /usr/bin/bash at {}; add bash to dependencies",
                bash.display()
            ),
        ));
    }

    let loader = build_dir.join("usr/lib/ld-linux-x86-64.so.2");
    if !loader.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "chroot build root has /usr/bin/bash but is missing /usr/lib/ld-linux-x86-64.so.2 at {}; add glibc or fix the standard chroot runtime",
                loader.display()
            ),
        ));
    }

    Ok(())
}

fn reset_build_root(
    base_dir: &str,
    paths: &BuildPaths,
    verbose: bool,
    reuse_rootfs: bool,
) -> io::Result<()> {
    if !reuse_rootfs && Path::new(base_dir).exists() {
        fs::remove_dir_all(base_dir)?;
    }
    fs::create_dir_all(base_dir)?;

    let work_dir = Path::new(base_dir).join(&paths.work);
    let out_dir = Path::new(base_dir).join(&paths.out);
    let tmp_dir = Path::new(base_dir).join("tmp");

    if reuse_rootfs {
        reset_reused_build_root(&work_dir, &out_dir, &tmp_dir)
    } else {
        recreate_build_dirs(verbose, &[&work_dir, &out_dir, &tmp_dir])
    }
}

fn reset_reused_build_root(work_dir: &Path, out_dir: &Path, tmp_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(work_dir)?;
    for dir in [out_dir, tmp_dir] {
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

fn recreate_build_dirs(verbose: bool, dirs: &[&Path]) -> io::Result<()> {
    for dir in dirs {
        if verbose {
            println!("Creating directory: {}", dir.display());
        }
        if dir.exists() {
            if verbose {
                println!("Removing existing {}", dir.display());
            }
            fs::remove_dir_all(dir)?;
        }
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

fn materialize_build_dependencies(
    base_dir: &str,
    repo_path: &str,
    fallback_repos: &[String],
    dependency_commits: &[String],
    manifest_dirs: &[PathBuf],
) -> io::Result<()> {
    let fallback_repo_paths: Vec<PathBuf> = fallback_repos.iter().map(PathBuf::from).collect();
    let requests: Vec<MaterializeRequest> = dependency_commits
        .iter()
        .map(|commit| materialize_request_for_build_dependency(commit.clone()))
        .collect();
    let config = MaterializeConfig {
        repo_path: repo_path.to_string(),
        target_dir: PathBuf::from(base_dir),
        mode: MaterializeMode::Flat,
        resolve_deps: true,
        manifest_db_paths: manifest_dirs.to_vec(),
        fallback_repo_paths,
        ..Default::default()
    };

    materialize(&config, &requests)?;
    Ok(())
}

fn create_usrmerge_symlinks(base_dir: &Path) -> io::Result<()> {
    for (link_path, target) in usrmerge_symlinks() {
        let full_link_path = base_dir.join(link_path);
        if !full_link_path.exists() {
            if let Some(parent) = full_link_path.parent() {
                fs::create_dir_all(parent)?;
            }
            std::os::unix::fs::symlink(target, &full_link_path)?;
        }
    }
    Ok(())
}

fn remove_existing_usrmerge_symlinks(build_dir: &Path) -> io::Result<()> {
    for (link_path, _) in usrmerge_symlinks() {
        let full_link_path = build_dir.join(link_path);
        if fs::symlink_metadata(&full_link_path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            fs::remove_file(&full_link_path)?;
        }
    }
    Ok(())
}

fn create_owned_usrmerge_symlinks(
    build_dir: &Path,
    mapped_uid: u32,
    mapped_gid: u32,
) -> io::Result<()> {
    for (link_path, target) in usrmerge_symlinks() {
        let full_link_path = build_dir.join(link_path);
        if let Some(parent) = full_link_path.parent() {
            fs::create_dir_all(parent)?;
            std::os::unix::fs::chown(parent, Some(mapped_uid), Some(mapped_gid))?;
        }
        if !full_link_path.exists() {
            std::os::unix::fs::symlink(target, &full_link_path)?;
        }
    }
    Ok(())
}

fn usrmerge_symlinks() -> [(&'static str, &'static str); 6] {
    [
        ("bin", "/usr/bin"),
        ("lib", "/usr/lib"),
        ("sbin", "/usr/bin"),
        ("lib64", "/usr/lib"),
        ("usr/lib64", "lib"),
        ("usr/sbin", "bin"),
    ]
}

#[cfg(test)]
#[path = "rootfs_tests.rs"]
mod rootfs_tests;
