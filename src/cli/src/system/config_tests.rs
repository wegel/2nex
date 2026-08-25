use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

use super::{capture_factory_defaults, move_legacy_etc_to_factory};

#[test]
fn nex_deployment_moves_legacy_etc_defaults_to_factory_tree() {
    let temp = tempfile::tempdir().expect("temp dir");
    let target = temp.path();
    let etc = target.join("etc");
    fs::create_dir_all(etc.join("ssh")).expect("etc tree");
    fs::write(etc.join("passwd"), "root:x:0:0\n").expect("passwd");
    fs::write(etc.join("ssh/sshd_config"), "PermitRootLogin no\n").expect("sshd config");
    fs::set_permissions(
        etc.join("ssh/sshd_config"),
        fs::Permissions::from_mode(0o640),
    )
    .expect("config mode");
    symlink("/proc/self/mounts", etc.join("mtab")).expect("mtab link");

    move_legacy_etc_to_factory(target, None).expect("move defaults");

    assert!(target.join("etc").is_dir());
    assert_eq!(fs::read_dir(target.join("etc")).unwrap().count(), 0);
    assert_eq!(
        fs::read_to_string(target.join("usr/share/factory/etc/passwd")).unwrap(),
        "root:x:0:0\n"
    );
    assert_eq!(
        fs::symlink_metadata(target.join("usr/share/factory/etc/ssh/sshd_config"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
    assert_eq!(
        fs::read_link(target.join("usr/share/factory/etc/mtab")).unwrap(),
        std::path::Path::new("/proc/self/mounts")
    );
}

#[test]
fn native_factory_defaults_and_legacy_defaults_merge_without_replacement() {
    let temp = tempfile::tempdir().expect("temp dir");
    let target = temp.path();
    fs::create_dir_all(target.join("etc/ssh")).expect("legacy etc");
    fs::create_dir_all(target.join("usr/share/factory/etc/ssh")).expect("factory etc");
    fs::write(target.join("etc/ssh/sshd_config"), "legacy\n").expect("legacy file");
    fs::write(target.join("usr/share/factory/etc/hosts"), "native\n").expect("native file");

    move_legacy_etc_to_factory(target, None).expect("merge defaults");

    assert_eq!(
        fs::read_to_string(target.join("usr/share/factory/etc/hosts")).unwrap(),
        "native\n"
    );
    assert_eq!(
        fs::read_to_string(target.join("usr/share/factory/etc/ssh/sshd_config")).unwrap(),
        "legacy\n"
    );
}

#[test]
fn conflicting_factory_and_legacy_defaults_fail_the_assembly() {
    let temp = tempfile::tempdir().expect("temp dir");
    let target = temp.path();
    fs::create_dir_all(target.join("etc")).expect("legacy etc");
    fs::create_dir_all(target.join("usr/share/factory/etc")).expect("factory etc");
    fs::write(target.join("etc/hosts"), "legacy\n").expect("legacy file");
    fs::write(target.join("usr/share/factory/etc/hosts"), "native\n").expect("native file");

    let error = move_legacy_etc_to_factory(target, None).expect_err("collision must fail");

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert!(error.to_string().contains("factory file"));
    assert_eq!(
        fs::read_to_string(target.join("usr/share/factory/etc/hosts")).unwrap(),
        "native\n"
    );
}

#[test]
fn broken_factory_symlink_still_blocks_a_legacy_default() {
    let temp = tempfile::tempdir().expect("temp dir");
    let target = temp.path();
    fs::create_dir_all(target.join("etc")).expect("legacy etc");
    fs::create_dir_all(target.join("usr/share/factory/etc")).expect("factory etc");
    fs::write(target.join("etc/masked"), "legacy\n").expect("legacy file");
    symlink(
        "/path-that-does-not-exist",
        target.join("usr/share/factory/etc/masked"),
    )
    .expect("factory link");

    let error = move_legacy_etc_to_factory(target, None).expect_err("symlink collision must fail");

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read_link(target.join("usr/share/factory/etc/masked")).unwrap(),
        std::path::Path::new("/path-that-does-not-exist")
    );
}

/// Scenario: a Nex child Assembly writes a new `/etc/shadow` after layering an
/// immutable base whose factory tree already contains that path.
///
/// Wanted behavior: the child's default replaces the base default in the
/// final factory tree. The immutable base output in Zub remains unchanged.
#[test]
fn child_default_replaces_the_base_factory_default() {
    let temp = tempfile::tempdir().expect("temp dir");
    let target = temp.path();
    fs::create_dir_all(target.join("etc")).expect("child etc");
    fs::create_dir_all(target.join("usr/share/factory/etc")).expect("base factory");
    fs::write(target.join("usr/share/factory/etc/shadow"), "base\n").expect("base default");
    let base_defaults = capture_factory_defaults(target).expect("capture base defaults");
    fs::write(target.join("etc/shadow"), "child\n").expect("child default");

    move_legacy_etc_to_factory(target, Some(&base_defaults)).expect("child factory merge");

    assert_eq!(
        fs::read_to_string(target.join("usr/share/factory/etc/shadow")).unwrap(),
        "child\n"
    );
    assert_eq!(fs::read_dir(target.join("etc")).unwrap().count(), 0);
}

/// Scenario: a child package changes a factory default that came from its
/// immutable base, then the child Assembly also writes the legacy `/etc` path.
///
/// Wanted behavior: Nex reports the child-owned collision. A base may be
/// replaced, but one child input must not silently erase another child input.
#[test]
fn child_package_change_prevents_base_default_replacement() {
    let temp = tempfile::tempdir().expect("temp dir");
    let target = temp.path();
    fs::create_dir_all(target.join("etc")).expect("child etc");
    fs::create_dir_all(target.join("usr/share/factory/etc")).expect("base factory");
    fs::write(target.join("usr/share/factory/etc/hosts"), "base\n").expect("base default");
    let base_defaults = capture_factory_defaults(target).expect("capture base defaults");
    fs::write(target.join("usr/share/factory/etc/hosts"), "package\n")
        .expect("child package default");
    fs::write(target.join("etc/hosts"), "assembly\n").expect("Assembly default");

    let error = move_legacy_etc_to_factory(target, Some(&base_defaults))
        .expect_err("two child inputs must conflict");

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read_to_string(target.join("usr/share/factory/etc/hosts")).unwrap(),
        "package\n"
    );
}
