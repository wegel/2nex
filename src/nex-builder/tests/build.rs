//! End-to-end tests for the small package builder.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use nex_builder::{build_graph, GraphOptions, OutputOptions};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use zub::ops::CheckoutOptions;
use zub::Repo;

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    manifest: PathBuf,
    repo: PathBuf,
    build: PathBuf,
    cache: PathBuf,
    dependency: String,
    environment_ref: String,
    source_hash: String,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temporary directory");
        let root = temp.path().to_path_buf();
        init_git(&root);
        let environment_ref = store_environment(&root);
        let source = root.join("fixtures/payload.txt");
        fs::create_dir(root.join("fixtures")).expect("create fixture directory");
        fs::write(&source, b"payload\n").expect("write source");
        let source_hash = sha256(b"payload\n");
        let repo = root.join("store");
        let dependency = seed_dependency(&repo, &root);

        Self {
            _temp: temp,
            manifest: root.join("pkg/test/package.yaml"),
            build: root.join("build"),
            cache: root.join("cache"),
            dependency,
            root,
            repo,
            environment_ref,
            source_hash,
        }
    }

    fn options(&self, check: bool) -> GraphOptions {
        let mut options = GraphOptions::serial(
            self.manifest.clone(),
            self.repo.clone(),
            self.build.clone(),
            self.cache.clone(),
            check,
        );
        options.output = OutputOptions::quiet();
        options
    }

    fn write_manifest(&self, script: &str, outputs: &str, source_hash: &str) {
        fs::create_dir_all(self.manifest.parent().expect("manifest parent"))
            .expect("create manifest directory");
        let indented_script = script
            .lines()
            .map(|line| format!("    {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let yaml = format!(
            r#"package:
  schema: 1
  name: Tiny
  slug: tiny
  namespace: tests/core
  version: 1.0+local
  description: Integration-test package
sources:
- name: payload-file
  file: fixtures/payload.txt
  sha256: {source_hash}
dependencies:
- commit: {dependency}
build:
  environment: {environment}
  script: |
{indented_script}
bundles:
  full: [bin, docs]
outputs:
{outputs}
"#,
            environment = self.environment_ref,
            dependency = self.dependency,
        );
        fs::write(&self.manifest, yaml).expect("write manifest");
    }
}

#[test]
fn builds_checks_partitions_and_publishes_one_package() {
    let fixture = Fixture::new();
    fixture.write_manifest(build_script(), normal_outputs(), &fixture.source_hash);

    let result = build_graph(&fixture.options(true)).expect("build package");
    let prefix = "x86_64/pkg/tests/core/tiny/1.0+local";
    assert_eq!(
        result.root_references,
        [
            format!("{prefix}/bundles/full"),
            format!("{prefix}/files"),
            format!("{prefix}/outputs/bin"),
            format!("{prefix}/outputs/docs"),
        ]
    );

    let repo = Repo::open(&fixture.repo).expect("open Zub repo");
    let checkout = fixture.root.join("checkout");
    zub::ops::checkout(
        &repo,
        &format!("{prefix}/bundles/full"),
        &checkout,
        CheckoutOptions::default(),
    )
    .expect("checkout bundle");
    assert_eq!(
        fs::read(checkout.join("usr/bin/hello")).expect("read bundled executable"),
        b"payload\n"
    );
    assert_eq!(
        fs::read(checkout.join("usr/share/doc/base")).expect("read bundled documentation"),
        b"dependency\n"
    );
    assert!(!checkout.join("tmp/scratch").exists());

    let raw = fixture.root.join("raw");
    zub::ops::checkout(
        &repo,
        &format!("{prefix}/files"),
        &raw,
        CheckoutOptions::default(),
    )
    .expect("checkout raw files");
    assert_eq!(
        fs::read(raw.join("tmp/scratch")).expect("read raw discarded file"),
        b"payload\n"
    );

    let hash = zub::read_ref(&repo, &format!("{prefix}/outputs/bin"))
        .expect("resolve published binary output");
    let commit = zub::read_commit(&repo, &hash).expect("read published binary output");
    assert_eq!(commit.metadata["nex.build.checksum"].len(), 64);
    assert!(zub::list_refs(&repo)
        .expect("list refs after publication")
        .iter()
        .all(|reference| !reference.starts_with("nex/tmp/")));
}

#[test]
fn keeps_undeclared_files_only_in_the_complete_files_ref() {
    let fixture = Fixture::new();
    fixture.write_manifest(build_script(), incomplete_outputs(), &fixture.source_hash);
    build_graph(&fixture.options(false)).expect("build package");

    let repo = Repo::open(&fixture.repo).expect("open package repo");
    let prefix = "x86_64/pkg/tests/core/tiny/1.0+local";
    let raw = fixture.root.join("raw-incomplete");
    zub::ops::checkout(
        &repo,
        &format!("{prefix}/files"),
        &raw,
        CheckoutOptions::default(),
    )
    .expect("checkout complete files ref");
    assert_eq!(
        fs::read(raw.join("tmp/scratch")).expect("read undeclared raw file"),
        b"payload\n"
    );

    let bundle = fixture.root.join("bundle-incomplete");
    zub::ops::checkout(
        &repo,
        &format!("{prefix}/bundles/full"),
        &bundle,
        CheckoutOptions::default(),
    )
    .expect("checkout output bundle");
    assert!(!bundle.join("tmp/scratch").exists());
}

#[test]
fn rejects_a_declared_output_that_the_build_did_not_create() {
    let fixture = Fixture::new();
    fixture.write_manifest(build_script(), missing_outputs(), &fixture.source_hash);

    let error = build_graph(&fixture.options(false)).unwrap_err();

    assert!(error.to_string().contains("declared output does not exist"));
}

#[test]
fn reuses_a_checksumless_package_by_its_recipe() {
    let fixture = Fixture::new();
    fixture.write_manifest(build_script(), normal_outputs(), &fixture.source_hash);
    assert_eq!(
        build_graph(&fixture.options(false))
            .expect("build checksumless package")
            .built,
        1
    );

    let source = fs::read_to_string(&fixture.manifest).expect("read package manifest");
    fs::write(
        &fixture.manifest,
        source.replace("  script: |", "  profile: [1:2]\n  script: |"),
    )
    .expect("add timing profile to manifest");
    let result = build_graph(&fixture.options(false)).expect("reuse unchanged recipe");

    assert_eq!((result.built, result.reused), (0, 1));

    fixture.write_manifest(
        &format!("{}\ntrue", build_script()),
        normal_outputs(),
        &fixture.source_hash,
    );
    assert_eq!(
        build_graph(&fixture.options(false))
            .expect("rebuild changed recipe")
            .built,
        1
    );
}

#[test]
fn adopts_complete_v3_refs_that_predate_recipe_metadata() {
    let fixture = Fixture::new();
    fixture.write_manifest(build_script(), normal_outputs(), &fixture.source_hash);
    build_graph(&fixture.options(false)).expect("publish refs with recipe metadata");

    let repo = Repo::open(&fixture.repo).expect("open package repo");
    let prefix = "x86_64/pkg/tests/core/tiny/1.0+local";
    for suffix in ["files", "outputs/bin", "outputs/docs"] {
        remove_recipe(&repo, &format!("{prefix}/{suffix}"));
    }
    remove_metadata(&repo, &format!("{prefix}/bundles/full"));
    let mut options = fixture.options(false);
    options.adopt_existing = true;
    let result = build_graph(&options).expect("adopt old complete refs");

    assert_eq!((result.built, result.reused), (0, 1));
    let hash =
        zub::resolve_ref(&repo, &format!("{prefix}/files")).expect("resolve adopted files ref");
    assert!(zub::read_commit(&repo, &hash)
        .expect("read adopted files commit")
        .metadata
        .contains_key("nex.build.recipe"));
}

#[test]
fn rebuilds_unkeyed_refs_unless_the_caller_adopts_them() {
    let fixture = Fixture::new();
    fixture.write_manifest(build_script(), normal_outputs(), &fixture.source_hash);
    build_graph(&fixture.options(false)).expect("publish refs with recipe metadata");

    let repo = Repo::open(&fixture.repo).expect("open package repo");
    let prefix = "x86_64/pkg/tests/core/tiny/1.0+local";
    for suffix in ["files", "outputs/bin", "outputs/docs"] {
        remove_recipe(&repo, &format!("{prefix}/{suffix}"));
    }
    remove_metadata(&repo, &format!("{prefix}/bundles/full"));

    let result = build_graph(&fixture.options(false)).expect("rebuild unkeyed refs");
    assert_eq!((result.built, result.reused), (1, 0));
}

#[test]
fn rejects_a_source_that_does_not_match_its_hash() {
    let fixture = Fixture::new();
    fixture.write_manifest(build_script(), normal_outputs(), &"0".repeat(64));

    let error = build_graph(&fixture.options(false)).unwrap_err();

    assert!(error.to_string().contains("source checksum mismatch"));
}

#[test]
fn refuses_to_clear_a_directory_it_did_not_create() {
    let fixture = Fixture::new();
    let node = fixture.build.join("nodes/00000");
    fs::create_dir_all(&node).expect("create unowned node directory");
    fs::write(node.join("keep"), b"human data").expect("write protected human data");
    fixture.write_manifest(build_script(), normal_outputs(), &fixture.source_hash);

    let error = build_graph(&fixture.options(false)).unwrap_err();

    assert!(error
        .to_string()
        .contains("refusing to clear unowned build directory"));
    assert_eq!(
        fs::read(node.join("keep")).expect("read protected human data"),
        b"human data"
    );
}

fn init_git(root: &Path) {
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(root)
        .status()
        .expect("run git init");
    assert!(status.success());
}

fn store_environment(root: &Path) -> String {
    let path = root.join("environment.yaml");
    fs::write(
        &path,
        r#"name: test
execution:
  chroot: false
paths:
  work: nex/work
  out: nex/out
  inputs: nex/inputs
env:
  PATH: /usr/bin:/bin
  SYSROOT: "{build_dir}"
  WORK_DIR: "{build_dir}/nex/work"
  OUT_DIR: "{build_dir}/nex/out"
preamble: ""
"#,
    )
    .expect("write build environment");
    let output = Command::new("git")
        .args(["hash-object", "-w", "environment.yaml"])
        .current_dir(root)
        .output()
        .expect("store Git blob");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("Git hash is UTF-8")
        .trim()
        .to_string()
}

fn seed_dependency(repo_path: &Path, root: &Path) -> String {
    let tree = root.join("dependency");
    fs::create_dir_all(tree.join("usr/share")).expect("create dependency tree");
    fs::write(tree.join("usr/share/base"), b"dependency\n").expect("write dependency file");
    let repo = Repo::init(repo_path).expect("initialize Zub repo");
    zub::ops::commit(&repo, &tree, "deps/base", None, Some("test"))
        .expect("commit dependency")
        .to_string()
}

fn remove_recipe(repo: &Repo, reference: &str) {
    let hash = zub::resolve_ref(repo, reference).expect("resolve ref before removing recipe");
    let commit = zub::read_commit(repo, &hash).expect("read commit before removing recipe");
    let checksum = commit.metadata["nex.build.checksum"].as_str();
    zub::ops::commit_tree_with_metadata(
        repo,
        &commit.tree,
        reference,
        Some(""),
        Some("test"),
        &[("nex.build.checksum", checksum)],
    )
    .expect("replace commit without recipe metadata");
}

fn remove_metadata(repo: &Repo, reference: &str) {
    let hash = zub::resolve_ref(repo, reference).expect("resolve ref before removing metadata");
    let commit = zub::read_commit(repo, &hash).expect("read commit before removing metadata");
    zub::ops::commit_tree_with_metadata(repo, &commit.tree, reference, Some(""), Some("test"), &[])
        .expect("replace commit without builder metadata");
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn build_script() -> &'static str {
    r#"test "$(cat "$SYSROOT/usr/share/base")" = dependency
install -Dm755 "$SOURCE_payload_file" "$OUT_DIR/usr/bin/hello"
install -Dm644 "$SYSROOT/usr/share/base" "$OUT_DIR/usr/share/doc/base"
install -Dm644 "$SOURCE_payload_file" "$OUT_DIR/tmp/scratch""#
}

fn normal_outputs() -> &'static str {
    r#"  bin:
    files:
    - path: /usr/bin/hello
  docs:
    files:
    - path: /usr/share/doc/base
  discard:
    files:
    - path: /tmp/scratch"#
}

fn incomplete_outputs() -> &'static str {
    r#"  bin:
    files:
    - path: /usr/bin/hello
  docs:
    files:
    - path: /usr/share/doc/base"#
}

fn missing_outputs() -> &'static str {
    r#"  bin:
    files:
    - path: /usr/bin/missing
  docs:
    files:
    - path: /usr/share/doc/base"#
}
