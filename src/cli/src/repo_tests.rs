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
