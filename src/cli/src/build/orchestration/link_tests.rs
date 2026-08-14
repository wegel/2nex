use std::fs;
use std::path::Path;
use std::process::Command;

use sha2::{Digest, Sha256};

use super::{link_manifest_dependencies, upsert_manifest_refs_in_section};
use crate::manifest::{load_manifest, ManifestData};

#[test]
fn link_pins_dependencies_and_assembly_packages_to_their_repository_revision() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let repository = temp_dir.path().join("product");
    let package_dir = repository.join("pkg/apps");
    let assembly_dir = repository.join("asm");
    fs::create_dir_all(&package_dir).expect("package dir");
    fs::create_dir_all(&assembly_dir).expect("assembly dir");
    git(&repository, &["init"]);
    git(&repository, &["config", "user.email", "test@example.test"]);
    git(&repository, &["config", "user.name", "Test"]);

    let patch_path = package_dir.join("demo.patch");
    fs::write(&patch_path, "committed patch\n").expect("patch");
    fs::write(
        package_dir.join("demo.yaml"),
        package_manifest(&sha256("committed patch\n")),
    )
    .expect("package manifest");
    let consumer_path = package_dir.join("consumer.yaml");
    fs::write(&consumer_path, consumer_manifest()).expect("consumer manifest");
    let assembly_path = assembly_dir.join("device.yaml");
    fs::write(&assembly_path, assembly_manifest()).expect("assembly manifest");
    git(&repository, &["add", "pkg", "asm"]);
    git(&repository, &["commit", "-m", "add manifests"]);
    let revision = git(&repository, &["rev-parse", "HEAD"]);

    link_manifest_dependencies(&consumer_path.to_string_lossy()).expect("link dependency");
    link_manifest_dependencies(&assembly_path.to_string_lossy()).expect("link assembly package");

    let ManifestData::Package(consumer) =
        load_manifest(&consumer_path.to_string_lossy()).expect("linked consumer")
    else {
        panic!("expected package manifest");
    };
    assert_eq!(
        consumer.dependencies[0].manifest_ref.as_deref(),
        Some(revision.as_str())
    );

    let ManifestData::System(assembly) =
        load_manifest(&assembly_path.to_string_lossy()).expect("linked assembly")
    else {
        panic!("expected system manifest");
    };
    assert_eq!(
        assembly.packages[0].manifest_ref.as_deref(),
        Some(revision.as_str())
    );

    fs::write(&patch_path, "working-tree patch\n").expect("changed patch");
    let error = link_manifest_dependencies(&consumer_path.to_string_lossy())
        .expect_err("dirty local source must not be pinned");
    assert!(error.to_string().contains("differs from HEAD"), "{error}");
}

#[test]
fn manifest_ref_updates_stay_in_the_requested_section_and_cover_duplicate_entries() {
    let mut manifest = r#"dependencies:
- name: shared
  commit: same/ref
  manifest_ref: old

packages:
- name: first
  manifest_ref: stale
  commit: same/ref
- name: second
  commit: same/ref

build:
  environment: abcdef
  script: "true"
"#
    .to_string();

    assert_eq!(
        upsert_manifest_refs_in_section(&mut manifest, "packages", "same/ref", "new"),
        2
    );
    assert!(manifest.contains("commit: same/ref\n  manifest_ref: old\n\npackages:"));
    assert_eq!(manifest.matches("manifest_ref: new").count(), 2);
}

fn package_manifest(patch_sha: &str) -> String {
    format!(
        r#"package:
  name: Demo
  slug: demo
  namespace: apps
  version: 1.0

sources:
- name: patch
  file: pkg/apps/demo.patch
  sha256: {patch_sha}

dependencies: []

build:
  environment: 0000000000000000000000000000000000000000
  script: "true"

bundles: {{}}
outputs: {{}}
"#
    )
}

fn consumer_manifest() -> &'static str {
    r#"package:
  name: Consumer
  slug: consumer
  namespace: apps
  version: 1.0

sources: []

dependencies:
- name: demo
  commit: x86_64/pkg/apps/demo/1.0/bundles/full

build:
  environment: 0000000000000000000000000000000000000000
  script: "true"

bundles: {}
outputs: {}
"#
}

fn assembly_manifest() -> &'static str {
    r#"system:
  name: Device
  slug: device
  version: 1.0

packages:
- name: demo
  commit: x86_64/pkg/apps/demo/1.0/bundles/full

build:
  environment: 0000000000000000000000000000000000000000
  script: "true"
"#
}

fn sha256(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn git(directory: &Path, args: &[&str]) -> String {
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
