use super::package_root_commits;

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
