use std::fs;
use std::process::Command;

use super::load_environment;

/// Build a Git repository holding an environment file, commit it, then change
/// the file so the committed blob is only reachable through history. This is
/// the shape every package manifest is in: 476 of them name a historical
/// revision of `env/standard.yaml`.
fn repository_with_historical_environment(root: &std::path::Path) -> String {
    let env_dir = root.join("env");
    fs::create_dir_all(&env_dir).expect("env dir");
    let env_file = env_dir.join("standard.yaml");
    fs::write(
        &env_file,
        "name: standard\npaths:\n  work: nex/work\n  out: nex/out\n  inputs: inputs\n",
    )
    .expect("env file");

    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git");
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "test"]);
    git(&["config", "user.email", "test@localhost"]);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "environment"]);

    let blob = Command::new("git")
        .args(["hash-object", "env/standard.yaml"])
        .current_dir(root)
        .output()
        .expect("hash-object");
    let sha = String::from_utf8_lossy(&blob.stdout).trim().to_string();

    // move the file on, so the recorded blob exists only in history
    fs::write(
        &env_file,
        "name: rewritten\npaths:\n  work: nex/work\n  out: nex/out\n  inputs: inputs\n",
    )
    .expect("rewrite");
    sha
}

#[test]
fn an_environment_blob_resolves_against_the_manifests_repository() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let manifests = temp_dir.path().join("manifests");
    fs::create_dir_all(&manifests).expect("manifests dir");
    let sha = repository_with_historical_environment(&manifests);

    // the store sits beside the manifests repository, exactly as on a machine,
    // and contains no Git objects
    let store = temp_dir.path().join("store");
    fs::create_dir_all(&store).expect("store dir");

    let manifest_path = manifests.join("pkg/demo.yaml");
    fs::create_dir_all(manifest_path.parent().unwrap()).expect("pkg dir");
    fs::write(&manifest_path, "package: {}\n").expect("manifest");

    let environment = load_environment(&manifest_path, &store.to_string_lossy(), &sha)
        .expect("a historical environment blob must resolve from the manifests repository");

    assert_eq!(environment.name, "standard");
}

#[test]
fn resolving_against_the_store_alone_would_fail() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let manifests = temp_dir.path().join("manifests");
    fs::create_dir_all(&manifests).expect("manifests dir");
    let sha = repository_with_historical_environment(&manifests);

    let store = temp_dir.path().join("store");
    fs::create_dir_all(&store).expect("store dir");

    // a manifest outside any repository falls back to the store, which is what
    // the old code always did; it must fail rather than silently succeed
    let stray = temp_dir.path().join("stray.yaml");
    fs::write(&stray, "package: {}\n").expect("stray manifest");

    let result = load_environment(&stray, &store.to_string_lossy(), &sha);

    assert!(
        result.is_err(),
        "the store holds no Git objects, so this must not resolve"
    );
}
