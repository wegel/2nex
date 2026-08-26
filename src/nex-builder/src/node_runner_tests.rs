//! Tests for graph-node reuse and log names.

use std::fs;

use zub::Repo;

use super::{log_filename, reuse_commits};

#[test]
fn declared_output_checksum_outweighs_recipe_changes() {
    let temp = tempfile::tempdir().expect("create reuse fixture directory");
    let source = temp.path().join("source");
    fs::create_dir(&source).expect("create reusable output");
    fs::write(source.join("file"), b"exact output").expect("write reusable output");
    let repo = Repo::init(&temp.path().join("repo")).expect("initialize reuse repo");
    zub::ops::commit_with_metadata(
        &repo,
        &source,
        "package/files",
        None,
        Some("test"),
        &[
            ("nex.build.checksum", "expected"),
            ("nex.build.recipe", "old"),
        ],
    )
    .expect("commit reusable output");

    assert!(reuse_commits(
        &repo,
        &["package/files".into()],
        Some("expected"),
        "new",
        false,
    )
    .expect("check checksum-based reuse"));
    assert!(
        !reuse_commits(&repo, &["package/files".into()], None, "new", false)
            .expect("check recipe-based reuse")
    );
}

#[test]
fn log_names_keep_the_node_identity_without_path_separators() {
    assert_eq!(
        log_filename(12, "package libs/system/glibc/2.39"),
        "00012-package-libs-system-glibc-2.39.log"
    );
}
