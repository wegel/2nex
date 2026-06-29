use super::write_root_commits;
use super::{
    create_symlink_forest_split, nex_paths, MaterializeConfig, MaterializeResult, PackageId,
    PackageMap, PackageSet, RuntimeClosure,
};

fn demo_package_id() -> PackageId {
    PackageId {
        path: "apps/demo/1.0".to_string(),
        manifest_hash: "abcdef1234567890".to_string(),
    }
}

#[test]
fn write_root_commits_appends_new_roots() {
    let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
    std::fs::write(
        temp_dir.path().join(".nex-app-root"),
        "x86_64/pkg/apps/demo/1.0/outputs/bin\n",
    )
    .expect("test setup should succeed");

    write_root_commits(
        temp_dir.path(),
        &[
            "x86_64/pkg/apps/demo/1.0/outputs/bin".to_string(),
            "x86_64/pkg/apps/demo/1.0/outputs/lib".to_string(),
        ],
    )
    .expect("test setup should succeed");

    let content = std::fs::read_to_string(temp_dir.path().join(".nex-app-root"))
        .expect("test setup should succeed");
    assert_eq!(
        content,
        "x86_64/pkg/apps/demo/1.0/outputs/bin\nx86_64/pkg/apps/demo/1.0/outputs/lib\n"
    );
}

#[test]
fn symlink_forest_uses_user_package_and_env_overrides() {
    let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
    let pkg_root = temp_dir.path().join("user/pkg");
    let env_root = temp_dir.path().join("user/env");
    let pkg_id = demo_package_id();
    let package_dir = pkg_root
        .join(&pkg_id.path)
        .join(&pkg_id.manifest_hash[..8])
        .join("usr/bin");
    std::fs::create_dir_all(&package_dir).expect("test setup should succeed");
    std::fs::write(package_dir.join("demo"), b"demo").expect("test setup should succeed");

    let config = MaterializeConfig {
        target_dir: temp_dir.path().join("logical-root"),
        pkg_dir_override: Some(pkg_root.clone()),
        env_dir_override: Some(env_root.clone()),
        ..Default::default()
    };
    let packages = PackageMap::from([(pkg_id.clone(), vec!["unused".to_string()])]);
    let root_packages = PackageSet::from([pkg_id]);
    let mut result = MaterializeResult::new(RuntimeClosure::default());
    let paths = nex_paths(&config).expect("test setup should succeed");

    create_symlink_forest_split(
        &paths.physical_pkg,
        &paths.logical_pkg,
        &paths.physical_env,
        &paths.logical_env,
        &packages,
        &root_packages,
        &mut result,
    )
    .expect("test setup should succeed");

    let link = env_root.join("bin/demo");
    let target = std::fs::read_link(&link).expect("test setup should succeed");
    let resolved = link
        .parent()
        .expect("test setup should succeed")
        .join(target);
    assert_eq!(
        resolved.canonicalize().expect("test setup should succeed"),
        package_dir.join("demo")
    );
    assert_eq!(result.symlinks_created, vec![link]);
}

#[test]
fn symlink_forest_points_staged_links_at_logical_packages() {
    let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
    let physical_pkg = temp_dir.path().join("upper/nex/pkg");
    let physical_env = temp_dir.path().join("upper/nex/env");
    let logical_pkg = temp_dir.path().join("final/nex/pkg");
    let logical_env = temp_dir.path().join("final/nex/env");
    let pkg_id = demo_package_id();
    let package_dir = physical_pkg
        .join(&pkg_id.path)
        .join(&pkg_id.manifest_hash[..8])
        .join("usr/bin");
    std::fs::create_dir_all(&package_dir).expect("test setup should succeed");
    std::fs::write(package_dir.join("demo"), b"demo").expect("test setup should succeed");

    let packages = PackageMap::from([(pkg_id.clone(), vec!["unused".to_string()])]);
    let root_packages = PackageSet::from([pkg_id]);
    let mut result = MaterializeResult::new(RuntimeClosure::default());

    create_symlink_forest_split(
        &physical_pkg,
        &logical_pkg,
        &physical_env,
        &logical_env,
        &packages,
        &root_packages,
        &mut result,
    )
    .expect("test setup should succeed");

    let physical_link = physical_env.join("bin/demo");
    let logical_link = logical_env.join("bin/demo");
    let target = std::fs::read_link(&physical_link).expect("test setup should succeed");
    assert_eq!(
        target,
        std::path::PathBuf::from("../../pkg/apps/demo/1.0/abcdef12/usr/bin/demo")
    );
    assert_eq!(result.symlinks_created, vec![logical_link]);
}
