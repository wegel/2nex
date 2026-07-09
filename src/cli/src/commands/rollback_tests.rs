//! Tests for system rollback command behavior.

use std::fs;
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::{copy_and_publish_rollback_with_command, run, RollbackArgs};

#[test]
fn rollback_publishes_temp_copy_atomically() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sysroot = build_sysroot(&temp_dir)?;
    let deployments = sysroot.join("nex/deployments");
    let old_checksum = checksum('a');
    let new_checksum = checksum('b');
    let old_deployment = deployments.join(format!("{}.0", old_checksum));
    let old_file = old_deployment.join("usr/bin/demo");
    let stale_temp = deployments.join(format!(".{}.2.tmp", old_checksum));

    create_deployment(&old_deployment, "old")?;
    create_deployment(&deployments.join(format!("{}.1", new_checksum)), "new")?;
    fs::create_dir_all(&stale_temp)?;
    fs::write(stale_temp.join("stale"), "stale")?;

    run(&RollbackArgs {
        sysroot: sysroot.clone(),
        to: None,
        yes: true,
        dry_run: false,
    })?;

    let rollback_file = deployments
        .join(format!("{}.2", old_checksum))
        .join("usr/bin/demo");
    assert!(!stale_temp.exists());
    assert_eq!(fs::read_to_string(&rollback_file)?, "old");
    assert_same_inode(&old_file, &rollback_file)?;
    Ok(())
}

#[test]
fn rollback_removes_temp_and_final_path_after_failed_copy() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let deployments = temp_dir.path().join("deployments");
    let src = deployments.join(format!("{}.0", checksum('a')));
    let temp = deployments.join(format!(".{}.1.tmp", checksum('a')));
    let dst = deployments.join(format!("{}.1", checksum('a')));
    let fake_cp = temp_dir.path().join("fake-cp");

    create_deployment(&src, "old")?;
    fs::create_dir_all(&temp)?;
    fs::write(
        &fake_cp,
        "#!/bin/sh\nprintf partial > \"$4/partial\"\nexit 1\n",
    )?;
    fs::set_permissions(&fake_cp, fs::Permissions::from_mode(0o755))?;

    let error = copy_and_publish_rollback_with_command(&fake_cp, &src, &temp, &dst)
        .expect_err("rollback should fail when the copy command fails");

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(!dst.exists());
    assert!(!temp.exists());
    Ok(())
}

fn build_sysroot(temp_dir: &TempDir) -> io::Result<PathBuf> {
    let sysroot = temp_dir.path().join("sysroot");
    fs::create_dir_all(sysroot.join("nex/deployments"))?;
    Ok(sysroot)
}

fn create_deployment(path: &Path, content: &str) -> io::Result<()> {
    fs::create_dir_all(path.join("usr/bin"))?;
    fs::write(path.join("usr/bin/demo"), content)
}

fn assert_same_inode(left: &Path, right: &Path) -> io::Result<()> {
    let left_metadata = fs::metadata(left)?;
    let right_metadata = fs::metadata(right)?;
    assert_eq!(
        (left_metadata.dev(), left_metadata.ino()),
        (right_metadata.dev(), right_metadata.ino())
    );
    Ok(())
}

fn checksum(fill: char) -> String {
    std::iter::repeat(fill).take(64).collect()
}
