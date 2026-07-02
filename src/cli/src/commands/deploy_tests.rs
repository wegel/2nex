use std::io;
use std::path::PathBuf;

use tempfile::TempDir;

use super::{run, DeployArgs};

#[test]
fn deploy_reaches_store_open_when_repo_and_deployments_use_different_devices() {
    let temp_dir = TempDir::new().expect("test setup should succeed");
    let sysroot = temp_dir.path().join("sysroot");
    let deployments = sysroot.join("nex/deployments");
    std::fs::create_dir_all(&deployments).expect("test setup should succeed");

    let error = run(&DeployArgs {
        system_ref: "systems/demo/0.0.1".to_string(),
        sysroot,
        repo: PathBuf::from("/proc"),
        dry_run: true,
    })
    .expect_err("deploy should fail because /proc is not a zub repo");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(
        error.to_string().contains("not a zub repository"),
        "unexpected error: {error}"
    );
}
