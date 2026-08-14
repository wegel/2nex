use std::fs;
use std::process::Command;

use super::{load_manifest, load_manifest_from_source};
use crate::manifest::{ManifestData, ManifestSource};
use crate::outputs::fetch_and_verify_input;

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
fn pinned_manifest_revision_uses_its_committed_local_source() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let upstream = temp_dir.path().join("product/upstream/nex");
    let manifest_path = upstream.join("pkg/apps/demo.yaml");
    let patch_path = upstream.join("pkg/apps/demo.patch");
    fs::create_dir_all(manifest_path.parent().unwrap()).expect("package dir");
    git(&upstream, &["init"]);
    git(&upstream, &["config", "user.email", "test@example.test"]);
    git(&upstream, &["config", "user.name", "Test"]);
    fs::write(&patch_path, "patch from revision\n").expect("pinned patch");
    let patch_sha = sha256("patch from revision\n");
    fs::write(
        &manifest_path,
        named_package_manifest("Pinned Name").replace("abcdef", &patch_sha),
    )
    .expect("pinned manifest");
    git(
        &upstream,
        &["add", "pkg/apps/demo.yaml", "pkg/apps/demo.patch"],
    );
    git(&upstream, &["commit", "-m", "add manifest"]);
    let revision = git(&upstream, &["rev-parse", "HEAD"]);
    fs::write(&manifest_path, named_package_manifest("Floating Name")).expect("floating manifest");
    fs::write(&patch_path, "patch from working tree\n").expect("floating patch");

    let loaded = load_manifest_from_source(&ManifestSource::Repository {
        revision,
        path: manifest_path,
        git_root: upstream,
    })
    .expect("pinned manifest revision");
    let ManifestData::Package(manifest) = loaded else {
        panic!("expected package manifest");
    };

    assert_eq!(manifest.package.name, "Pinned Name");
    let download_dir = temp_dir.path().join("downloads");
    fs::create_dir(&download_dir).expect("download dir");
    let staged = fetch_and_verify_input(&manifest.sources[0], &download_dir.to_string_lossy())
        .expect("committed local source");
    assert_eq!(fs::read_to_string(staged).unwrap(), "patch from revision\n");
}

#[test]
fn pinned_assembly_is_rejected_instead_of_reading_overlays_from_the_working_tree() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let repository = temp_dir.path().join("product");
    let manifest_path = repository.join("asm/device.yaml");
    fs::create_dir_all(manifest_path.parent().unwrap()).expect("assembly dir");
    git(&repository, &["init"]);
    git(&repository, &["config", "user.email", "test@example.test"]);
    git(&repository, &["config", "user.name", "Test"]);
    fs::write(
        &manifest_path,
        r#"system:
  name: Device
  slug: device
  version: 1
packages:
- commit: x86_64/pkg/apps/demo/1/bundles/full
build:
  environment: abcdef
  script: "true"
"#,
    )
    .expect("assembly manifest");
    git(&repository, &["add", "asm/device.yaml"]);
    git(&repository, &["commit", "-m", "add assembly"]);
    let revision = git(&repository, &["rev-parse", "HEAD"]);

    let error = match load_manifest_from_source(&ManifestSource::Repository {
        revision,
        path: manifest_path,
        git_root: repository,
    }) {
        Ok(_) => panic!("pinned assembly must not float local inputs"),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("pin the assembly repository checkout"));
}

fn sha256(value: &str) -> String {
    use sha2::{Digest, Sha256};

    hex::encode(Sha256::digest(value.as_bytes()))
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
