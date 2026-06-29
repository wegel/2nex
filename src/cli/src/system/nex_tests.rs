use std::collections::HashMap;
use std::io;

use super::install_package_commit;

#[test]
fn invalid_package_ref_stops_nex_materialization() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let mut installed_packages = HashMap::new();

    let error = install_package_commit(
        "unused-repo",
        temp_dir.path(),
        &temp_dir.path().join("nex/pkg"),
        "not-a-package-ref",
        &mut installed_packages,
    )
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("not-a-package-ref"));
    assert!(installed_packages.is_empty());
}
