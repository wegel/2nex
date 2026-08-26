//! Tests for moving mutable `/etc` defaults into a Nex image.

use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

use super::{capture, move_etc};

#[test]
fn moves_files_modes_and_links_into_factory_etc() {
    let temporary = tempfile::tempdir().expect("create factory fixture directory");
    let target = temporary.path();
    fs::create_dir_all(target.join("etc/ssh")).expect("create mutable etc tree");
    fs::write(target.join("etc/ssh/config"), "default\n").expect("write factory default");
    fs::set_permissions(
        target.join("etc/ssh/config"),
        fs::Permissions::from_mode(0o640),
    )
    .expect("set factory default mode");
    symlink("/proc/self/mounts", target.join("etc/mtab")).expect("create etc symlink");

    move_etc(target, None).expect("move etc into factory defaults");

    assert_eq!(
        fs::read_dir(target.join("etc"))
            .expect("read emptied etc")
            .count(),
        0
    );
    assert_eq!(
        fs::metadata(target.join("usr/share/factory/etc/ssh/config"))
            .expect("read factory default metadata")
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
    assert_eq!(
        fs::read_link(target.join("usr/share/factory/etc/mtab")).expect("read factory symlink"),
        std::path::Path::new("/proc/self/mounts")
    );
}

#[test]
fn refuses_to_replace_a_factory_file_from_the_same_assembly() {
    let temporary = tempfile::tempdir().expect("create factory fixture directory");
    let target = temporary.path();
    fs::create_dir_all(target.join("etc")).expect("create mutable etc");
    fs::create_dir_all(target.join("usr/share/factory/etc")).expect("create factory etc");
    fs::write(target.join("etc/hosts"), "legacy\n").expect("write mutable hosts");
    fs::write(target.join("usr/share/factory/etc/hosts"), "native\n").expect("write factory hosts");

    let error = move_etc(target, None).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert!(error.to_string().contains("factory file"));
}

#[test]
fn child_can_replace_an_unchanged_base_default() {
    let temporary = tempfile::tempdir().expect("create factory fixture directory");
    let target = temporary.path();
    fs::create_dir_all(target.join("etc")).expect("create child etc");
    fs::create_dir_all(target.join("usr/share/factory/etc")).expect("create base factory etc");
    fs::write(target.join("usr/share/factory/etc/shadow"), "base\n")
        .expect("write base factory default");
    let base = capture(target).expect("capture base factory defaults");
    fs::write(target.join("etc/shadow"), "child\n").expect("write child default");

    move_etc(target, Some(&base)).expect("replace unchanged base default");

    assert_eq!(
        fs::read_to_string(target.join("usr/share/factory/etc/shadow"))
            .expect("read child factory default"),
        "child\n"
    );
}
