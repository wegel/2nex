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
    let temp_dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        temp_dir.path().join(".nex-app-root"),
        "x86_64/pkg/apps/demo/1.0/outputs/bin\n",
    )
    .unwrap();

    write_root_commits(
        temp_dir.path(),
        &[
            "x86_64/pkg/apps/demo/1.0/outputs/bin".to_string(),
            "x86_64/pkg/apps/demo/1.0/outputs/lib".to_string(),
        ],
    )
    .unwrap();

    let content = std::fs::read_to_string(temp_dir.path().join(".nex-app-root")).unwrap();
    assert_eq!(
        content,
        "x86_64/pkg/apps/demo/1.0/outputs/bin\nx86_64/pkg/apps/demo/1.0/outputs/lib\n"
    );
}

#[test]
fn symlink_forest_uses_user_package_and_env_overrides() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let pkg_root = temp_dir.path().join("user/pkg");
    let env_root = temp_dir.path().join("user/env");
    let pkg_id = demo_package_id();
    let package_dir = pkg_root
        .join(&pkg_id.path)
        .join(&pkg_id.manifest_hash[..8])
        .join("usr/bin");
    std::fs::create_dir_all(&package_dir).unwrap();
    std::fs::write(package_dir.join("demo"), b"demo").unwrap();

    let config = MaterializeConfig {
        target_dir: temp_dir.path().join("logical-root"),
        pkg_dir_override: Some(pkg_root.clone()),
        env_dir_override: Some(env_root.clone()),
        ..Default::default()
    };
    let packages = PackageMap::from([(pkg_id.clone(), vec!["unused".to_string()])]);
    let root_packages = PackageSet::from([pkg_id]);
    let mut result = MaterializeResult::new(RuntimeClosure::default());
    let paths = nex_paths(&config).unwrap();

    create_symlink_forest_split(
        &paths.physical_pkg,
        &paths.logical_pkg,
        &paths.physical_env,
        &paths.logical_env,
        &packages,
        &root_packages,
        &mut result,
    )
    .unwrap();

    let link = env_root.join("bin/demo");
    let target = std::fs::read_link(&link).unwrap();
    let resolved = link.parent().unwrap().join(target);
    assert_eq!(resolved.canonicalize().unwrap(), package_dir.join("demo"));
    assert_eq!(result.symlinks_created, vec![link]);
}
