use std::fs;
use std::io;
use std::path::PathBuf;

use tempfile::TempDir;

use super::{
    materialize_request_for_build_dependency, refresh_chroot_usrmerge_symlinks,
    validate_chroot_build_root,
};
use crate::materializer::MaterializeRequest;

#[test]
fn refreshes_chroot_usrmerge_symlinks() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();
    fs::create_dir_all(root.join("usr"))?;
    std::os::unix::fs::symlink("old-lib", root.join("usr/lib64"))?;

    refresh_chroot_usrmerge_symlinks(root)?;

    assert_eq!(fs::read_link(root.join("bin"))?, PathBuf::from("/usr/bin"));
    assert_eq!(fs::read_link(root.join("lib"))?, PathBuf::from("/usr/lib"));
    assert_eq!(fs::read_link(root.join("sbin"))?, PathBuf::from("/usr/bin"));
    assert_eq!(
        fs::read_link(root.join("lib64"))?,
        PathBuf::from("/usr/lib")
    );
    assert_eq!(fs::read_link(root.join("usr/lib64"))?, PathBuf::from("lib"));
    assert_eq!(fs::read_link(root.join("usr/sbin"))?, PathBuf::from("bin"));

    Ok(())
}

#[test]
fn classifies_build_dependency_materialize_requests() {
    let bundle = materialize_request_for_build_dependency(
        "x86_64/pkg/cli/shells/bash/5.2.21/bundles/dev".to_string(),
    );
    let output = materialize_request_for_build_dependency(
        "x86_64/pkg/libs/system/glibc/2.39/outputs/lib".to_string(),
    );

    assert!(matches!(bundle, MaterializeRequest::Bundle { .. }));
    assert!(matches!(output, MaterializeRequest::Output { .. }));
}

#[test]
fn validates_chroot_build_root_launcher_files() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();
    fs::create_dir_all(root.join("usr/bin"))?;
    fs::write(root.join("usr/bin/bash"), b"")?;

    let err = validate_chroot_build_root(root).expect_err("loader should be required");
    assert!(err
        .to_string()
        .contains("missing /usr/lib/ld-linux-x86-64.so.2"));

    fs::create_dir_all(root.join("usr/lib"))?;
    fs::write(root.join("usr/lib/ld-linux-x86-64.so.2"), b"")?;
    validate_chroot_build_root(root)?;

    Ok(())
}
