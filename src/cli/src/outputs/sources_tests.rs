use std::fs;
use std::process::Command;

use super::sources::fetch_and_verify_input;
use crate::manifest::types::Source;

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git");
    assert!(
        status.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&status.stderr)
    );
}

/// A repository whose committed content is later changed, so the commit is only
/// reachable through history. That is the shape a bundle has to preserve.
fn repository_with_history(root: &std::path::Path) -> String {
    fs::create_dir_all(root).expect("repo dir");
    fs::write(root.join("env.yaml"), "name: first\n").expect("first");
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "test"]);
    git(root, &["config", "user.email", "test@localhost"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "first"]);

    fs::write(root.join("env.yaml"), "name: second\n").expect("second");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "second"]);

    let head = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("rev-parse");
    String::from_utf8_lossy(&head.stdout).trim().to_string()
}

fn bundle_source(commit: &str, sha256: Option<&str>) -> Source {
    Source {
        name: "snapshot".to_string(),
        url: None,
        file: None,
        dev: None,
        cargo_lock: None,
        cargo_toml: None,
        go_sum: None,
        zig_zon: None,
        git_bundle: Some(commit.to_string()),
        sha256: sha256.map(str::to_string),
        repository_snapshot: None,
    }
}

#[test]
fn a_git_bundle_source_is_byte_reproducible_and_carries_history() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let repo = temp_dir.path().join("repo");
    let head = repository_with_history(&repo);
    let downloads = temp_dir.path().join("downloads");
    fs::create_dir_all(&downloads).expect("downloads");

    let previous = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&repo).expect("enter repo");

    // the first build does not know the checksum, so it reports the real one
    let discovered = fetch_and_verify_input(&bundle_source(&head, Some(&"0".repeat(64))), &downloads.to_string_lossy())
        .expect_err("a wrong checksum must be rejected")
        .to_string();
    let actual = discovered
        .split("got ")
        .nth(1)
        .expect("the error names the real checksum")
        .trim()
        .to_string();

    let first = fetch_and_verify_input(&bundle_source(&head, Some(&actual)), &downloads.to_string_lossy())
        .expect("bundle builds");
    fs::remove_file(&first).expect("drop the cached bundle");
    let second = fetch_and_verify_input(&bundle_source(&head, Some(&actual)), &downloads.to_string_lossy())
        .expect("bundle rebuilds");

    std::env::set_current_dir(previous).expect("restore cwd");

    assert_eq!(
        fs::read(&first).unwrap_or_default().len(),
        fs::read(&second).expect("second bundle").len(),
        "two builds of one commit must produce the same bundle"
    );

    // the point of a bundle over a tarball: cloning it yields real history, so a
    // blob that only exists in an earlier commit is still reachable
    let clone_dir = temp_dir.path().join("clone");
    let cloned = Command::new("git")
        .args([
            "clone",
            "--quiet",
            &second.to_string_lossy(),
            &clone_dir.to_string_lossy(),
        ])
        .output()
        .expect("git clone");
    assert!(
        cloned.status.success(),
        "cloning the bundle failed: {}",
        String::from_utf8_lossy(&cloned.stderr)
    );

    let count = Command::new("git")
        .arg("-C")
        .arg(&clone_dir)
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .expect("rev-list");
    assert_eq!(
        String::from_utf8_lossy(&count.stdout).trim(),
        "2",
        "the clone must carry both commits, not just the tip"
    );

    let historical = Command::new("git")
        .arg("-C")
        .arg(&clone_dir)
        .args(["cat-file", "-t", "HEAD~1:env.yaml"])
        .output()
        .expect("cat-file");
    assert!(
        historical.status.success(),
        "the superseded revision must still resolve from the clone"
    );
}
