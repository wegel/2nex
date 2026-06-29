//! Nex package capsule materialization for system roots.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::refs::PackageRef;
use crate::store::{checkout_into, get_commit_id, get_commit_metadata};

use super::flat::fresh_target_dir;
use super::nex_db::{deploy_manifests_to_nex_db, flatten_package_dependencies};
use super::nex_links::{
    create_file_symlinks_recursive, create_target_fhs_symlinks, symlink_flattened_libs_to_usr,
};
use super::nex_shim::install_nex_ld_shim;

pub(super) struct PackageInstall {
    namespace: String,
    slug: String,
    version: String,
    commit: String,
    checksum: String,
    install_dir: PathBuf,
}

/// Materialize packages into `/nex/pkg` capsules plus FHS symlinks.
pub fn materialize_nex_structure(
    base_dir: &str,
    repo_path: &str,
    package_commits: &[String],
) -> io::Result<()> {
    let target_dir = fresh_target_dir(base_dir)?;
    let nex_pkg_dir = target_dir.join("nex/pkg");
    let lib64_dir = target_dir.join("lib64");
    fs::create_dir_all(&nex_pkg_dir)?;
    fs::create_dir_all(&lib64_dir)?;
    deploy_manifests_to_nex_db(&target_dir)?;

    let mut installed_packages = HashMap::new();
    for commit in package_commits {
        install_package_commit(
            repo_path,
            &target_dir,
            &nex_pkg_dir,
            commit,
            &mut installed_packages,
        )?;
    }

    flatten_capsules_if_manifest_db_exists(repo_path, &nex_pkg_dir, &target_dir)?;
    symlink_flattened_libs_to_usr(&nex_pkg_dir, &target_dir)?;
    install_nex_ld_shim(repo_path, &lib64_dir)?;
    create_target_fhs_symlinks(&target_dir)?;

    println!(
        "Materialized {} packages to /nex/pkg/ structure",
        installed_packages.len()
    );
    Ok(())
}

fn install_package_commit(
    repo_path: &str,
    target_dir: &Path,
    nex_pkg_dir: &Path,
    commit: &str,
    installed_packages: &mut HashMap<String, String>,
) -> io::Result<()> {
    let Some(package_ref) = parse_package_ref(commit) else {
        return Ok(());
    };
    if is_kernel_module_output(commit) {
        return install_kernel_modules(repo_path, target_dir, &package_ref, commit);
    }

    let install = package_install(repo_path, nex_pkg_dir, commit, &package_ref)?;
    print_package_install(
        &install,
        installed_packages.contains_key(&package_key(&install)),
    );
    checkout_package_output(repo_path, &install)?;
    link_package_directories(target_dir, &install)?;
    installed_packages.insert(package_key(&install), commit.to_string());
    Ok(())
}

fn parse_package_ref(commit: &str) -> Option<PackageRef> {
    match PackageRef::parse(commit) {
        Ok(package_ref) => Some(package_ref),
        Err(e) => {
            eprintln!("Warning: could not parse package commit {}: {}", commit, e);
            None
        }
    }
}

fn install_kernel_modules(
    repo_path: &str,
    target_dir: &Path,
    package_ref: &PackageRef,
    commit: &str,
) -> io::Result<()> {
    println!(
        "Installing kernel modules: {}/{}/{} (direct layer)",
        package_ref.namespace, package_ref.slug, package_ref.version
    );
    checkout_into(repo_path, commit, target_dir, true)
}

fn package_install(
    repo_path: &str,
    nex_pkg_dir: &Path,
    commit: &str,
    package_ref: &PackageRef,
) -> io::Result<PackageInstall> {
    let base_commit = package_ref.commit_ref();
    let manifest_hash = get_manifest_hash(repo_path, &base_commit)?;
    let checksum = manifest_hash[..8.min(manifest_hash.len())].to_string();
    let install_dir = nex_pkg_dir
        .join(&package_ref.namespace)
        .join(&package_ref.slug)
        .join(&package_ref.version)
        .join(&checksum);

    Ok(PackageInstall {
        namespace: package_ref.namespace.clone(),
        slug: package_ref.slug.clone(),
        version: package_ref.version.clone(),
        commit: commit.to_string(),
        checksum,
        install_dir,
    })
}

fn get_manifest_hash(repo_path: &str, base_commit: &str) -> io::Result<String> {
    match get_commit_metadata(repo_path, base_commit, "nex.manifest.hash") {
        Ok(hash) => Ok(hash),
        Err(_) => Ok(get_commit_id(repo_path, base_commit).unwrap_or_default()),
    }
}

fn print_package_install(install: &PackageInstall, already_installed: bool) {
    if already_installed {
        println!(
            "  Merging output into {}/{}/{} ({})",
            install.namespace, install.slug, install.version, install.checksum
        );
    } else {
        println!(
            "Installing package: {}/{}/{}",
            install.namespace, install.slug, install.version
        );
        println!(
            "  Using manifest hash: {} ({})",
            install.commit, install.checksum
        );
    }
}

fn checkout_package_output(repo_path: &str, install: &PackageInstall) -> io::Result<()> {
    if let Some(parent) = install.install_dir.parent() {
        fs::create_dir_all(parent)?;
    }
    checkout_into(repo_path, &install.commit, &install.install_dir, true)?;
    record_package_root_commit(install)?;
    Ok(())
}

fn record_package_root_commit(install: &PackageInstall) -> io::Result<()> {
    let sentinel_path = install.install_dir.join(".nex-app-root");
    let mut commits = if sentinel_path.exists() {
        fs::read_to_string(&sentinel_path)?
    } else {
        String::new()
    };
    if !commits.lines().any(|commit| commit == install.commit) {
        commits.push_str(&install.commit);
        commits.push('\n');
        fs::write(sentinel_path, commits)?;
    }
    Ok(())
}

fn link_package_directories(target_dir: &Path, install: &PackageInstall) -> io::Result<()> {
    for entry in fs::read_dir(&install.install_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        let pkg_dir = install.install_dir.join(&dir_name);
        let target_subdir = target_dir.join(&dir_name);

        fs::create_dir_all(&target_subdir)?;
        create_file_symlinks_recursive(&pkg_dir, &target_subdir, install, &dir_name)?;
    }
    Ok(())
}

fn flatten_capsules_if_manifest_db_exists(
    repo_path: &str,
    nex_pkg_dir: &Path,
    target_dir: &Path,
) -> io::Result<()> {
    let nex_db_pkg = target_dir.join("nex/db/pkg");
    if nex_db_pkg.exists() {
        flatten_package_dependencies(repo_path, nex_pkg_dir, &nex_db_pkg)
    } else {
        println!("  Skipping dependency flattening: /nex/db/pkg not found");
        Ok(())
    }
}

fn package_key(install: &PackageInstall) -> String {
    format!("{}/{}", install.namespace, install.slug)
}

fn is_kernel_module_output(commit: &str) -> bool {
    let is_boot_output = commit.contains("/outputs/boot");
    commit.contains("/kernel/linux/") && commit.contains("/outputs/") && !is_boot_output
}

pub(super) fn package_symlink_target(
    install: &PackageInstall,
    relative_base: &str,
    rel_path: &Path,
) -> String {
    format!(
        "/nex/pkg/{}/{}/{}/{}/{}/{}",
        install.namespace,
        install.slug,
        install.version,
        install.checksum,
        relative_base,
        rel_path.display()
    )
}
