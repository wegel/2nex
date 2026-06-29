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
