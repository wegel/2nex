//! Tests for system deployment command behavior.

use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::{run, DeployArgs};
use crate::store::{commit_tree, Store};

#[test]
fn deploy_reaches_store_open_when_repo_and_deployments_use_different_devices() {
    let temp_dir = TempDir::new().expect("test setup should succeed");
    let sysroot = temp_dir.path().join("sysroot");
    let deployments = sysroot.join("nex/deployments");
    std::fs::create_dir_all(&deployments).expect("test setup should succeed");

    let error = run(&DeployArgs {
        system_ref: "systems/demo/0.0.1".to_string(),
        sysroot,
        repo: Some(PathBuf::from("/proc")),
        dry_run: true,
        force: false,
        allow_commit_hash: false,
    })
    .expect_err("deploy should fail because /proc is not a zub repo");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(
        error.to_string().contains("not a zub repository"),
        "unexpected error: {error}"
    );
}

#[test]
fn deploy_requires_system_checksum_metadata_by_default() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let Some(repo) = build_test_repo(&temp_dir, &[])? else {
        return Ok(());
    };
    let sysroot = build_sysroot(&temp_dir)?;

    let error = run(&DeployArgs {
        system_ref: "systems/demo/0.0.1".to_string(),
        sysroot,
        repo: Some(repo),
        dry_run: true,
        force: false,
        allow_commit_hash: false,
    })
    .expect_err("deploy should reject a system ref without checksum metadata");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("nex.system.checksum"));
    assert!(error.to_string().contains("nex.build.checksum"));
    Ok(())
}

#[test]
fn deploy_accepts_build_checksum_metadata() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let checksum = checksum('d');
    let Some(repo) = build_test_repo(
        &temp_dir,
        &[("nex.build.checksum".to_string(), checksum.clone())],
    )?
    else {
        return Ok(());
    };
    let sysroot = build_sysroot(&temp_dir)?;

    run(&DeployArgs {
        system_ref: "systems/demo/0.0.1".to_string(),
        sysroot: sysroot.clone(),
        repo: Some(repo),
        dry_run: false,
        force: false,
        allow_commit_hash: false,
    })?;

    assert!(sysroot
        .join("nex/deployments")
        .join(format!("{}.1", checksum))
        .is_dir());
    Ok(())
}

#[test]
fn deploy_rejects_existing_checksum_without_force() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let checksum = checksum('a');
    let Some(repo) = build_test_repo(
        &temp_dir,
        &[("nex.system.checksum".to_string(), checksum.clone())],
    )?
    else {
        return Ok(());
    };
    let sysroot = build_sysroot(&temp_dir)?;
    fs::create_dir_all(
        sysroot
            .join("nex/deployments")
            .join(format!("{}.0", checksum)),
    )?;

    let error = run(&DeployArgs {
        system_ref: "systems/demo/0.0.1".to_string(),
        sysroot,
        repo: Some(repo),
        dry_run: true,
        force: false,
        allow_commit_hash: false,
    })
    .expect_err("deploy should reject a duplicate checksum");

    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert!(error.to_string().contains("--force"));
    Ok(())
}

#[test]
fn deploy_publishes_temp_checkout_atomically() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let checksum = checksum('b');
    let Some(repo) = build_test_repo(
        &temp_dir,
        &[("nex.system.checksum".to_string(), checksum.clone())],
    )?
    else {
        return Ok(());
    };
    let sysroot = build_sysroot(&temp_dir)?;
    let deployments_dir = sysroot.join("nex/deployments");
    let temp_deployment = deployments_dir.join(format!(".{}.1.tmp", checksum));
    fs::create_dir_all(&temp_deployment)?;
    fs::write(temp_deployment.join("stale"), "stale")?;

    run(&DeployArgs {
        system_ref: "systems/demo/0.0.1".to_string(),
        sysroot,
        repo: Some(repo),
        dry_run: false,
        force: false,
        allow_commit_hash: false,
    })?;

    assert!(!temp_deployment.exists());
    assert_eq!(
        fs::read_to_string(
            deployments_dir
                .join(format!("{}.1", checksum))
                .join("usr/bin/demo"),
        )?,
        "demo"
    );
    Ok(())
}

#[test]
fn deploy_allows_duplicate_checksum_with_force() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let checksum = checksum('c');
    let Some(repo) = build_test_repo(
        &temp_dir,
        &[("nex.system.checksum".to_string(), checksum.clone())],
    )?
    else {
        return Ok(());
    };
    let sysroot = build_sysroot(&temp_dir)?;
    let deployments_dir = sysroot.join("nex/deployments");
    fs::create_dir_all(deployments_dir.join(format!("{}.0", checksum)))?;

    run(&DeployArgs {
        system_ref: "systems/demo/0.0.1".to_string(),
        sysroot,
        repo: Some(repo),
        dry_run: false,
        force: true,
        allow_commit_hash: false,
    })?;

    assert!(deployments_dir.join(format!("{}.1", checksum)).is_dir());
    Ok(())
}

#[test]
fn deploy_hardlinks_from_repo_outside_sysroot() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let checksum = checksum('e');
    let Some(repo) = build_test_repo(
        &temp_dir,
        &[("nex.system.checksum".to_string(), checksum.clone())],
    )?
    else {
        return Ok(());
    };
    let sysroot = build_sysroot(&temp_dir)?;

    run(&DeployArgs {
        system_ref: "systems/demo/0.0.1".to_string(),
        sysroot: sysroot.clone(),
        repo: Some(repo.clone()),
        dry_run: false,
        force: false,
        allow_commit_hash: false,
    })?;

    let deployed_file = sysroot
        .join("nex/deployments")
        .join(format!("{}.1", checksum))
        .join("usr/bin/demo");
    let repo_blob = find_samefile_blob(&repo, &deployed_file)?
        .expect("deployed file should share an inode with a repo blob");
    let deployed_metadata = fs::metadata(&deployed_file)?;
    let blob_metadata = fs::metadata(&repo_blob)?;

    assert_eq!(deployed_metadata.dev(), blob_metadata.dev());
    assert_eq!(deployed_metadata.ino(), blob_metadata.ino());
    assert!(deployed_metadata.nlink() > 1);
    Ok(())
}

fn build_sysroot(temp_dir: &TempDir) -> io::Result<PathBuf> {
    let sysroot = temp_dir.path().join("sysroot");
    fs::create_dir_all(sysroot.join("nex/deployments"))?;
    Ok(sysroot)
}

fn build_test_repo(
    temp_dir: &TempDir,
    metadata: &[(String, String)],
) -> io::Result<Option<PathBuf>> {
    let repo = temp_dir.path().join("repo");
    if !init_test_store(&repo)? {
        return Ok(None);
    }

    let tree = temp_dir.path().join("tree");
    fs::create_dir_all(tree.join("usr/bin"))?;
    fs::write(tree.join("usr/bin/demo"), "demo")?;

    if !commit_test_tree(&repo, "systems/demo/0.0.1", &tree, metadata)? {
        return Ok(None);
    }

    Ok(Some(repo))
}

fn init_test_store(repo_path: &Path) -> io::Result<bool> {
    match Store::init(repo_path) {
        Ok(_) => Ok(true),
        Err(error) if host_lacks_root_user_namespace_mapping(&error) => {
            eprintln!("skipping deploy test: host cannot map uid 0");
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

fn commit_test_tree(
    repo_path: &Path,
    ref_name: &str,
    tree_dir: &Path,
    metadata: &[(String, String)],
) -> io::Result<bool> {
    match commit_tree(
        &repo_path.display().to_string(),
        ref_name,
        tree_dir,
        metadata,
    ) {
        Ok(_) => Ok(true),
        Err(error) if host_lacks_root_user_namespace_mapping(&error) => {
            eprintln!("skipping deploy test: host cannot map uid 0");
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

fn host_lacks_root_user_namespace_mapping(error: &io::Error) -> bool {
    error.to_string().contains("uid 0 not mapped in namespace")
}

fn checksum(fill: char) -> String {
    std::iter::repeat(fill).take(64).collect()
}

fn find_samefile_blob(repo: &Path, deployed_file: &Path) -> io::Result<Option<PathBuf>> {
    let deployed_metadata = fs::metadata(deployed_file)?;
    let blob_root = repo.join("objects/blobs");

    for entry in walkdir::WalkDir::new(blob_root) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let metadata = entry.metadata()?;
        if metadata.dev() == deployed_metadata.dev() && metadata.ino() == deployed_metadata.ino() {
            return Ok(Some(entry.path().to_path_buf()));
        }
    }

    Ok(None)
}
