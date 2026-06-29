use super::write_root_commits;

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
