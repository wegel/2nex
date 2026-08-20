use std::path::PathBuf;

use super::prefer_local_manifest_dirs;

#[test]
fn local_checkout_does_not_mix_with_installed_manifests() {
    let local = vec![
        PathBuf::from("product/pkg"),
        PathBuf::from("upstream/nex/pkg"),
    ];
    let installed = vec![PathBuf::from("/nex/db/pkg")];

    assert_eq!(prefer_local_manifest_dirs(local.clone(), installed), local);
}

#[test]
fn installed_manifests_are_used_outside_a_checkout() {
    let installed = vec![PathBuf::from("/nex/db/pkg")];

    assert_eq!(
        prefer_local_manifest_dirs(Vec::new(), installed.clone()),
        installed
    );
}

/// Both store paths exist as mount points in a built image, so preferring the
/// new name by existence alone picked an empty directory over the machine's
/// real store and every operation failed with "not a zub repository".
#[test]
fn an_empty_store_directory_does_not_win_over_a_real_one() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let empty = temp_dir.path().join("store");
    let real = temp_dir.path().join("repo");
    std::fs::create_dir_all(&empty).expect("empty mount point");
    std::fs::create_dir_all(real.join("objects")).expect("objects");
    std::fs::create_dir_all(real.join("refs")).expect("refs");

    assert!(!super::is_store(&empty), "a bare mount point is not a store");
    assert!(super::is_store(&real), "objects plus refs make a store");
}
