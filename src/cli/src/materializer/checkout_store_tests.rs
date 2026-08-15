use std::os::unix::fs::{symlink, PermissionsExt};

use super::checkout_one_file;

#[test]
fn checkout_one_file_preserves_absolute_symlink_entries() {
    let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
    let checkout_dir = temp_dir.path().join("checkout");
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir_all(checkout_dir.join("usr/lib")).expect("test setup should succeed");
    symlink(
        "/nex/pkg/libs/demo/1.0/abcdef/usr/lib/libdemo.so",
        checkout_dir.join("usr/lib/libdemo.so"),
    )
    .expect("test setup should succeed");

    checkout_one_file(
        &checkout_dir,
        "/usr/lib/libdemo.so",
        &target_dir,
        "x86_64/pkg/libs/demo/1.0/abcdef/files",
    )
    .expect("test setup should succeed");

    let copied_link = target_dir.join("usr/lib/libdemo.so");
    let link_target = std::fs::read_link(copied_link).expect("test setup should succeed");
    assert_eq!(
        link_target,
        std::path::PathBuf::from("/nex/pkg/libs/demo/1.0/abcdef/usr/lib/libdemo.so")
    );
}

#[test]
fn checkout_one_file_copies_chained_relative_symlink_targets() {
    let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
    let checkout_dir = temp_dir.path().join("checkout");
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir_all(checkout_dir.join("usr/lib/real")).expect("test setup should succeed");
    std::fs::write(checkout_dir.join("usr/lib/real/libdemo.so"), "demo")
        .expect("test setup should succeed");
    symlink(
        "real/libdemo.so",
        checkout_dir.join("usr/lib/libdemo-real.so"),
    )
    .expect("test setup should succeed");
    symlink("libdemo-real.so", checkout_dir.join("usr/lib/libdemo.so"))
        .expect("test setup should succeed");

    checkout_one_file(
        &checkout_dir,
        "/usr/lib/libdemo.so",
        &target_dir,
        "x86_64/pkg/libs/demo/1.0/abcdef/files",
    )
    .expect("test setup should succeed");

    assert_eq!(
        std::fs::read_link(target_dir.join("usr/lib/libdemo.so"))
            .expect("test setup should succeed"),
        std::path::PathBuf::from("libdemo-real.so")
    );
    assert_eq!(
        std::fs::read_link(target_dir.join("usr/lib/libdemo-real.so"))
            .expect("test setup should succeed"),
        std::path::PathBuf::from("real/libdemo.so")
    );
    assert_eq!(
        std::fs::read_to_string(target_dir.join("usr/lib/real/libdemo.so"))
            .expect("test setup should succeed"),
        "demo"
    );
}

#[test]
fn checkout_one_file_creates_nested_relative_symlink_target_parent() {
    let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
    let checkout_dir = temp_dir.path().join("checkout");
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir_all(checkout_dir.join("usr/lib/nested"))
        .expect("test setup should succeed");
    std::fs::write(checkout_dir.join("usr/lib/nested/libdemo.so"), "demo")
        .expect("test setup should succeed");
    symlink("nested/libdemo.so", checkout_dir.join("usr/lib/libdemo.so"))
        .expect("test setup should succeed");

    checkout_one_file(
        &checkout_dir,
        "/usr/lib/libdemo.so",
        &target_dir,
        "x86_64/pkg/libs/demo/1.0/abcdef/files",
    )
    .expect("test setup should succeed");

    assert_eq!(
        std::fs::read_to_string(target_dir.join("usr/lib/nested/libdemo.so"))
            .expect("test setup should succeed"),
        "demo"
    );
}

#[test]
fn checkout_one_file_replaces_a_read_only_regular_file() {
    let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
    let checkout_dir = temp_dir.path().join("checkout");
    let target_dir = temp_dir.path().join("target");
    let source = checkout_dir.join("usr/lib/libdemo.so");
    let target = target_dir.join("usr/lib/libdemo.so");

    std::fs::create_dir_all(source.parent().expect("source should have a parent"))
        .expect("test setup should succeed");
    std::fs::create_dir_all(target.parent().expect("target should have a parent"))
        .expect("test setup should succeed");
    std::fs::write(&source, "new content").expect("test setup should succeed");
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o444))
        .expect("test setup should succeed");
    std::fs::write(&target, "old content").expect("test setup should succeed");
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444))
        .expect("test setup should succeed");

    checkout_one_file(
        &checkout_dir,
        "/usr/lib/libdemo.so",
        &target_dir,
        "x86_64/pkg/libs/demo/1.0/abcdef/files",
    )
    .expect("read-only destination should be replaced");

    assert_eq!(
        std::fs::read_to_string(&target).expect("target should be readable"),
        "new content"
    );
    assert_eq!(
        std::fs::metadata(&target)
            .expect("target metadata should be readable")
            .permissions()
            .mode()
            & 0o777,
        0o444
    );
    assert_eq!(
        std::fs::read_to_string(&source).expect("source should be readable"),
        "new content"
    );
}

#[test]
fn checkout_one_file_replaces_a_symlink_without_writing_its_target() {
    let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
    let checkout_dir = temp_dir.path().join("checkout");
    let target_dir = temp_dir.path().join("target");
    let source = checkout_dir.join("usr/lib/libdemo.so");
    let target = target_dir.join("usr/lib/libdemo.so");
    let old_target = target_dir.join("old-target");

    std::fs::create_dir_all(source.parent().expect("source should have a parent"))
        .expect("test setup should succeed");
    std::fs::create_dir_all(target.parent().expect("target should have a parent"))
        .expect("test setup should succeed");
    std::fs::write(&source, "new content").expect("test setup should succeed");
    std::fs::write(&old_target, "keep this").expect("test setup should succeed");
    symlink(&old_target, &target).expect("test setup should succeed");

    checkout_one_file(
        &checkout_dir,
        "/usr/lib/libdemo.so",
        &target_dir,
        "x86_64/pkg/libs/demo/1.0/abcdef/files",
    )
    .expect("symlink destination should be replaced");

    assert!(!std::fs::symlink_metadata(&target)
        .expect("target metadata should be readable")
        .file_type()
        .is_symlink());
    assert_eq!(
        std::fs::read_to_string(&target).expect("target should be readable"),
        "new content"
    );
    assert_eq!(
        std::fs::read_to_string(&old_target).expect("old target should be readable"),
        "keep this"
    );
}
