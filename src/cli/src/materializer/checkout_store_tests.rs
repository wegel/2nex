use std::os::unix::fs::symlink;

use super::checkout_one_file;

#[test]
fn checkout_one_file_preserves_absolute_symlink_entries() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let checkout_dir = temp_dir.path().join("checkout");
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir_all(checkout_dir.join("usr/lib")).unwrap();
    symlink(
        "/nex/pkg/libs/demo/1.0/abcdef/usr/lib/libdemo.so",
        checkout_dir.join("usr/lib/libdemo.so"),
    )
    .unwrap();

    checkout_one_file(
        &checkout_dir,
        "/usr/lib/libdemo.so",
        &target_dir,
        "x86_64/pkg/libs/demo/1.0/abcdef/files",
    )
    .unwrap();

    let copied_link = target_dir.join("usr/lib/libdemo.so");
    let link_target = std::fs::read_link(copied_link).unwrap();
    assert_eq!(
        link_target,
        std::path::PathBuf::from("/nex/pkg/libs/demo/1.0/abcdef/usr/lib/libdemo.so")
    );
}

#[test]
fn checkout_one_file_copies_chained_relative_symlink_targets() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let checkout_dir = temp_dir.path().join("checkout");
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir_all(checkout_dir.join("usr/lib/real")).unwrap();
    std::fs::write(checkout_dir.join("usr/lib/real/libdemo.so"), "demo").unwrap();
    symlink(
        "real/libdemo.so",
        checkout_dir.join("usr/lib/libdemo-real.so"),
    )
    .unwrap();
    symlink("libdemo-real.so", checkout_dir.join("usr/lib/libdemo.so")).unwrap();

    checkout_one_file(
        &checkout_dir,
        "/usr/lib/libdemo.so",
        &target_dir,
        "x86_64/pkg/libs/demo/1.0/abcdef/files",
    )
    .unwrap();

    assert_eq!(
        std::fs::read_link(target_dir.join("usr/lib/libdemo.so")).unwrap(),
        std::path::PathBuf::from("libdemo-real.so")
    );
    assert_eq!(
        std::fs::read_link(target_dir.join("usr/lib/libdemo-real.so")).unwrap(),
        std::path::PathBuf::from("real/libdemo.so")
    );
    assert_eq!(
        std::fs::read_to_string(target_dir.join("usr/lib/real/libdemo.so")).unwrap(),
        "demo"
    );
}

#[test]
fn checkout_one_file_creates_nested_relative_symlink_target_parent() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let checkout_dir = temp_dir.path().join("checkout");
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir_all(checkout_dir.join("usr/lib/nested")).unwrap();
    std::fs::write(checkout_dir.join("usr/lib/nested/libdemo.so"), "demo").unwrap();
    symlink("nested/libdemo.so", checkout_dir.join("usr/lib/libdemo.so")).unwrap();

    checkout_one_file(
        &checkout_dir,
        "/usr/lib/libdemo.so",
        &target_dir,
        "x86_64/pkg/libs/demo/1.0/abcdef/files",
    )
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(target_dir.join("usr/lib/nested/libdemo.so")).unwrap(),
        "demo"
    );
}
