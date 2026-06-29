use std::os::unix::fs::symlink;

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
