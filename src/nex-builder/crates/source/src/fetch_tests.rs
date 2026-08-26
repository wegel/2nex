//! Tests for Git source bundles.

use std::fs;

use super::{git_bundle, git_status};

#[test]
fn git_bundles_are_repeatable_and_cloneable() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository).unwrap();
    git_status(&repository, &["init", "--quiet"], &[]).unwrap();
    fs::write(repository.join("payload"), b"payload\n").unwrap();
    git_status(&repository, &["add", "payload"], &[]).unwrap();
    git_status(
        &repository,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "test",
        ],
        &[],
    )
    .unwrap();
    let manifest = repository.join("manifest.yaml");
    let first = temporary.path().join("first.bundle");
    let second = temporary.path().join("second.bundle");
    git_bundle(&manifest, "HEAD", &first).unwrap();
    git_bundle(&manifest, "HEAD", &second).unwrap();
    assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
    let clone = temporary.path().join("clone");
    git_status(
        temporary.path(),
        &["clone", "--quiet"],
        &[first.as_os_str(), clone.as_os_str()],
    )
    .unwrap();
    assert_eq!(fs::read(clone.join("payload")).unwrap(), b"payload\n");
}
