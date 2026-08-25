use std::fs;

use super::{deploy_manifests_to_nex_db, package_root_commits};

#[test]
fn package_root_commits_reads_all_root_outputs() {
    let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
    std::fs::write(
        temp_dir.path().join(".nex-app-root"),
        "x86_64/pkg/apps/demo/1.0/outputs/bin\nx86_64/pkg/apps/demo/1.0/outputs/lib\n",
    )
    .expect("test setup should succeed");

    let commits = package_root_commits(temp_dir.path()).expect("test setup should succeed");

    assert_eq!(
        commits,
        vec![
            "x86_64/pkg/apps/demo/1.0/outputs/bin".to_string(),
            "x86_64/pkg/apps/demo/1.0/outputs/lib".to_string(),
        ]
    );
}

#[test]
fn deployment_combines_product_and_upstream_manifest_trees() {
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let product = temp_dir.path().join("product/pkg/apps");
    let upstream = temp_dir.path().join("upstream/pkg/libs");
    let target = temp_dir.path().join("target");
    fs::create_dir_all(&product).expect("product package dir");
    fs::create_dir_all(&upstream).expect("upstream package dir");
    fs::write(product.join("agent.yaml"), manifest("agent", "apps")).expect("product manifest");
    fs::write(upstream.join("runtime.yaml"), manifest("runtime", "libs"))
        .expect("upstream manifest");

    deploy_manifests_to_nex_db(
        &target,
        &[
            temp_dir.path().join("product/pkg"),
            temp_dir.path().join("upstream/pkg"),
        ],
        false,
    )
    .expect("manifest deployment");

    assert!(target.join("nex/db/pkg/apps/agent.yaml").is_file());
    assert!(target.join("nex/db/pkg/libs/runtime.yaml").is_file());
}

/// Scenario: a Nex-structured child layers a base that already carries a manifest database.
/// Nex must replace that database with the current checkout instead of reporting collisions.
#[test]
fn deployment_replaces_a_base_manifest_database() {
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let packages = temp_dir.path().join("source/pkg/apps");
    let target = temp_dir.path().join("target");
    fs::create_dir_all(&packages).expect("package dir");
    fs::create_dir_all(target.join("nex/db/pkg/old")).expect("base database");
    fs::write(target.join("nex/db/pkg/old/stale.yaml"), "stale\n").expect("base manifest");
    fs::write(packages.join("agent.yaml"), manifest("agent", "apps")).expect("current manifest");

    deploy_manifests_to_nex_db(&target, &[temp_dir.path().join("source/pkg")], true)
        .expect("manifest deployment");

    assert!(!target.join("nex/db/pkg/old/stale.yaml").exists());
    assert!(target.join("nex/db/pkg/apps/agent.yaml").is_file());
}

fn manifest(slug: &str, namespace: &str) -> String {
    format!(
        "package:\n  name: {slug}\n  slug: {slug}\n  namespace: {namespace}\n  version: 1.0\n\nsources: []\ndependencies: []\n\nbuild:\n  environment: abcdef\n  script: \"true\"\n\nbundles: {{}}\noutputs: {{}}\n"
    )
}
