use std::fs;
use std::process::Command;

use super::{load_manifest, load_manifest_from_source};
use crate::manifest::{ManifestData, ManifestSource};

#[test]
fn package_local_sources_resolve_from_their_owning_repository() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let repository = temp_dir.path().join("product");
    let manifest_path = repository.join("pkg/apps/demo.yaml");
    let patch_path = repository.join("pkg/apps/demo.patch");
    fs::create_dir_all(repository.join(".git")).expect("git marker");
    fs::create_dir_all(manifest_path.parent().unwrap()).expect("package dir");
    fs::write(&patch_path, "patch").expect("local source");
    fs::write(&manifest_path, package_manifest()).expect("package manifest");

    let loaded = load_manifest(&manifest_path.to_string_lossy()).expect("loaded manifest");
    let ManifestData::Package(manifest) = loaded else {
        panic!("expected package manifest");
    };

    assert_eq!(manifest.sources[0].file.as_deref(), patch_path.to_str());
    assert_eq!(
        manifest.sources[1].cargo_lock.as_deref(),
        Some("https://example.test/Cargo.lock")
    );
}

#[test]
fn pinned_manifest_blob_uses_the_git_repository_that_owns_it() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let upstream = temp_dir.path().join("product/upstream/nex");
    let manifest_path = upstream.join("pkg/apps/demo.yaml");
    fs::create_dir_all(manifest_path.parent().unwrap()).expect("package dir");
    git(&upstream, &["init"]);
    git(&upstream, &["config", "user.email", "test@example.test"]);
    git(&upstream, &["config", "user.name", "Test"]);
    fs::write(&manifest_path, named_package_manifest("Pinned Name")).expect("pinned manifest");
    git(&upstream, &["add", "pkg/apps/demo.yaml"]);
    git(&upstream, &["commit", "-m", "add manifest"]);
    let sha = git(&upstream, &["rev-parse", "HEAD:pkg/apps/demo.yaml"]);
    fs::write(&manifest_path, named_package_manifest("Floating Name")).expect("floating manifest");

    let loaded = load_manifest_from_source(&ManifestSource::Blob {
        sha,
        path: manifest_path,
        git_root: upstream,
    })
    .expect("pinned manifest blob");
    let ManifestData::Package(manifest) = loaded else {
        panic!("expected package manifest");
    };

    assert_eq!(manifest.package.name, "Pinned Name");
}

fn package_manifest() -> &'static str {
    r#"package:
  name: demo
  slug: demo
  namespace: apps
  version: 1.0

sources:
- name: patch
  file: pkg/apps/demo.patch
  sha256: abcdef
- name: cargo
  cargo_lock: https://example.test/Cargo.lock
  sha256: abcdef

dependencies: []

build:
  environment: abcdef
  script: "true"

bundles: {}
outputs: {}
"#
}

fn named_package_manifest(name: &str) -> String {
    package_manifest().replacen("name: demo", &format!("name: {name}"), 1)
}

fn git(directory: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .output()
        .expect("git command");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
