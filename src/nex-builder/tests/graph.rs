//! End-to-end tests for graph planning, scheduling, reuse, and assemblies.

use std::fs;
use std::num::NonZeroUsize;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use nex_builder::{build_graph, ChecksumKind, Error, GraphOptions, OutputOptions};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use zub::ops::CheckoutOptions;
use zub::Repo;

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    repo: PathBuf,
    build: PathBuf,
    cache: PathBuf,
    sync: PathBuf,
    environment: String,
    source_url: String,
    source_hash: String,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temporary directory");
        let root = temp.path().to_path_buf();
        init_git(&root);
        fs::create_dir_all(root.join("catalog/pkg/tests")).expect("create package catalog");
        fs::create_dir_all(root.join("catalog/assemblies/fixtures"))
            .expect("create assembly catalog");
        let sync = root.join("sync");
        fs::create_dir(&sync).expect("create graph synchronization directory");
        let source = root.join("source.txt");
        fs::write(&source, b"shared source\n").expect("write graph source fixture");
        let environment = store_environment(&root, &sync);
        Self {
            _temp: temp,
            repo: root.join("store"),
            build: root.join("build"),
            cache: root.join("cache"),
            sync,
            environment,
            source_url: format!("file://{}", source.display()),
            source_hash: format!("{:x}", Sha256::digest(b"shared source\n")),
            root,
        }
    }

    fn options(&self, manifest: impl AsRef<Path>, jobs: usize, cpus: usize) -> GraphOptions {
        GraphOptions {
            manifest: self.root.join(manifest),
            repo: self.repo.clone(),
            build_dir: self.build.clone(),
            source_cache: self.cache.clone(),
            check: false,
            adopt_existing: false,
            jobs: NonZeroUsize::new(jobs).expect("nonzero test worker count"),
            cpus: NonZeroUsize::new(cpus).expect("nonzero test CPU count"),
            output: OutputOptions::quiet(),
        }
    }

    fn write_package(
        &self,
        slug: &str,
        dependencies: &[&str],
        script: &str,
        outputs: &[&str],
        checksum: Option<&str>,
    ) {
        let dependencies = if dependencies.is_empty() {
            "dependencies: []".to_string()
        } else {
            let entries = dependencies
                .iter()
                .map(|reference| format!("- commit: {reference}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("dependencies:\n{entries}")
        };
        let outputs = outputs
            .iter()
            .map(|path| format!("    - path: {path}"))
            .collect::<Vec<_>>()
            .join("\n");
        let checksum = checksum.map_or(String::new(), |value| format!("  checksum: {value}\n"));
        let script = indent(script, 4);
        let yaml = format!(
            "package:\n  schema: 1\n  name: {slug}\n  slug: {slug}\n  namespace: tests\n  version: 1.0\n  description: Graph test package\n{checksum}sources:\n- name: shared\n  url: {source_url}\n  sha256: {source_hash}\n{dependencies}\nbuild:\n  environment: {environment}\n  script: |\n{script}\noutputs:\n  out:\n    files:\n{outputs}\n",
            environment = self.environment,
            source_url = self.source_url,
            source_hash = self.source_hash,
        );
        fs::write(
            self.root.join(format!("catalog/pkg/tests/{slug}.yaml")),
            yaml,
        )
        .expect("write package manifest");
    }

    fn checksum(&self, reference: &str) -> String {
        let repo = Repo::open(&self.repo).expect("open graph repo");
        let hash = zub::resolve_ref(&repo, reference).expect("resolve graph result ref");
        zub::read_commit(&repo, &hash)
            .expect("read graph result commit")
            .metadata["nex.build.checksum"]
            .clone()
    }
}

#[test]
fn builds_parallel_package_graph_and_assembly_then_reuses_it() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root.join("catalog/assemblies/fixtures/config.txt"),
        b"from source\n",
    )
    .expect("write assembly source fixture");
    write_success_graph(&fixture, None);

    let first = build_graph(&fixture.options("catalog/assemblies/system.yaml", 2, 3))
        .expect("build complete graph");
    assert_eq!((first.built, first.reused), (5, 0));
    assert_eq!(first.root_references, ["systems/test-system/1.0"]);
    assert!(fixture
        .cache
        .join(format!("sha256-{}", fixture.source_hash))
        .is_file());

    let checksums = graph_checksums(&fixture);
    write_success_graph(&fixture, Some(&checksums));
    let second = build_graph(&fixture.options("catalog/assemblies/system.yaml", 2, 3))
        .expect("reuse complete graph");
    assert_eq!((second.built, second.reused), (0, 5));
    verify_system(&fixture);
}

#[test]
fn builds_and_checks_out_selected_paths_from_raw_package_files() {
    let fixture = Fixture::new();
    fixture.write_package(
        "tools",
        &[],
        "install -Dm755 /dev/null \"$OUT_DIR/usr/bin/ldd\"\ninstall -Dm755 /dev/null \"$OUT_DIR/usr/bin/getent\"\ninstall -Dm755 /dev/null \"$OUT_DIR/usr/bin/unused\"",
        &["/usr/bin/ldd", "/usr/bin/getent", "/usr/bin/unused"],
        None,
    );
    let assembly = format!(
        "system:\n  schema: 1\n  name: Selected system\n  slug: selected-system\n  version: 1.0\n  description: Graph test assembly\npackages:\n- commit: x86_64/pkg/tests/tools/1.0/files\n  paths:\n  - /usr/bin/ldd\n  - /usr/bin/getent\nbuild:\n  environment: {}\n  script: |\n    test -x \"$SYSROOT/target/usr/bin/ldd\"\n    test -x \"$SYSROOT/target/usr/bin/getent\"\n    test ! -e \"$SYSROOT/target/usr/bin/unused\"\n",
        fixture.environment
    );
    fs::write(
        fixture
            .root
            .join("catalog/assemblies/fixtures/selected.yaml"),
        assembly,
    )
    .expect("write selected-path assembly");

    let result = build_graph(&fixture.options("catalog/assemblies/fixtures/selected.yaml", 2, 2))
        .expect("build selected-path graph");
    assert_eq!((result.built, result.reused), (2, 0));

    let repo = Repo::open(&fixture.repo).expect("open selected-path repo");
    let system = fixture.root.join("selected-system");
    zub::ops::checkout(
        &repo,
        "systems/selected-system/1.0",
        &system,
        CheckoutOptions::default(),
    )
    .expect("checkout selected-path system");
    assert!(system.join("usr/bin/ldd").is_file());
    assert!(system.join("usr/bin/getent").is_file());
    assert!(!system.join("usr/bin/unused").exists());
    assert_eq!(
        fs::metadata(system.join("usr/bin/ldd"))
            .expect("read selected tool mode")
            .permissions()
            .mode()
            & 0o777,
        0o755
    );

    let raw = fixture.root.join("raw-tools");
    zub::ops::checkout(
        &repo,
        "x86_64/pkg/tests/tools/1.0/files",
        &raw,
        CheckoutOptions::default(),
    )
    .expect("checkout raw tool package");
    assert!(raw.join("usr/bin/unused").is_file());
}

#[test]
fn rejects_a_cycle_before_any_build_script_runs() {
    let fixture = Fixture::new();
    fixture.write_package("a", &[&package_ref("b")], marker_script("a"), &["/a"], None);
    fixture.write_package("b", &[&package_ref("a")], marker_script("b"), &["/b"], None);

    let error = build_graph(&fixture.options("catalog/pkg/tests/a.yaml", 2, 2)).unwrap_err();

    assert!(matches!(error, Error::GraphCycle { .. }));
    assert!(!fixture.sync.join("a").exists());
    assert!(!fixture.sync.join("b").exists());
}

#[test]
fn classifies_an_unsupported_manifest_schema() {
    let fixture = Fixture::new();
    fixture.write_package("bad-schema", &[], "true", &["/unused"], None);
    let path = fixture.root.join("catalog/pkg/tests/bad-schema.yaml");
    let yaml = fs::read_to_string(&path).expect("read package manifest");
    fs::write(&path, yaml.replace("schema: 1", "schema: 999")).expect("replace package schema");

    let error = build_graph(&fixture.options(&path, 1, 1)).expect_err("reject package schema");

    assert!(matches!(
        error,
        Error::UnsupportedSchema { version: 999, .. }
    ));
}

#[test]
fn classifies_an_invalid_dependency_reference() {
    let fixture = Fixture::new();
    fixture.write_package("bad-ref", &["mutable-ref"], "true", &["/unused"], None);

    let error = build_graph(&fixture.options("catalog/pkg/tests/bad-ref.yaml", 1, 1))
        .expect_err("reject dependency reference");

    assert!(matches!(
        error,
        Error::InvalidReference { value } if value == "mutable-ref"
    ));
}

#[test]
fn classifies_a_missing_stored_input() {
    let fixture = Fixture::new();
    let hash = "a".repeat(64);
    fixture.write_package("missing", &[&hash], "true", &["/unused"], None);

    let error = build_graph(&fixture.options("catalog/pkg/tests/missing.yaml", 1, 1))
        .expect_err("reject missing stored input");

    assert!(
        matches!(
            &error,
            Error::MissingInput { reference, .. } if reference == &hash
        ),
        "{error:?}"
    );
}

#[test]
fn keeps_checksum_class_inside_the_failed_node() {
    let fixture = Fixture::new();
    fixture.write_package(
        "wrong-checksum",
        &[],
        "install -Dm644 /dev/null \"$OUT_DIR/file\"",
        &["/file"],
        Some(&"0".repeat(64)),
    );

    let error = build_graph(&fixture.options("catalog/pkg/tests/wrong-checksum.yaml", 1, 1))
        .expect_err("reject output checksum");
    let Error::NodeFailed { source, .. } = error else {
        panic!("checksum failure lost its node context")
    };

    assert!(matches!(
        *source,
        Error::ChecksumMismatch {
            kind: ChecksumKind::PackageOutput,
            ..
        }
    ));
}

#[test]
fn stops_new_nodes_after_failure_and_waits_for_running_nodes() {
    let fixture = Fixture::new();
    fixture.write_package(
        "fail",
        &[],
        "touch \"$SYNC/fail\"\necho deliberate failure\nexit 9",
        &["/fail"],
        None,
    );
    fixture.write_package(
        "slow",
        &[],
        "sleep 0.1\ntouch \"$SYNC/slow\"\ninstall -Dm644 /dev/null \"$OUT_DIR/slow\"",
        &["/slow"],
        None,
    );
    fixture.write_package(
        "blocked",
        &[&package_ref("fail"), &package_ref("slow")],
        marker_script("blocked"),
        &["/blocked"],
        None,
    );

    let error = build_graph(&fixture.options("catalog/pkg/tests/blocked.yaml", 2, 2)).unwrap_err();

    assert!(matches!(&error, Error::NodeFailed { .. }));
    assert!(error.to_string().contains("package tests/fail/1.0 failed"));
    assert!(error.to_string().contains("build log:"));
    let failed_log = fs::read_dir(fixture.build.join("logs"))
        .expect("read build logs")
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .ends_with("package-tests-fail-1.0.log")
        })
        .expect("readable failed-package log");
    let log = fs::read_to_string(failed_log.path()).expect("read failed-package log");
    assert!(log.contains("== build 1 =="));
    assert!(log.contains("deliberate failure"));
    assert!(fixture.sync.join("slow").is_file());
    assert!(!fixture.sync.join("blocked").exists());
}

struct Checksums {
    a: String,
    b: String,
    combined: String,
    base: String,
    system: String,
}

fn write_success_graph(fixture: &Fixture, checksums: Option<&Checksums>) {
    let leaf = |name: &str, other: &str| {
        format!(
            "touch \"$SYNC/{name}\"\nfor unused in $(seq 1 200); do test -e \"$SYNC/{other}\" && break; sleep 0.01; done\ntest -e \"$SYNC/{other}\"\nprintf '%s\\n' \"$MAKEFLAGS\" > \"$OUT_DIR/{name}\""
        )
    };
    fixture.write_package(
        "a",
        &[],
        &leaf("a", "b"),
        &["/a"],
        checksums.map(|c| c.a.as_str()),
    );
    fixture.write_package(
        "b",
        &[],
        &leaf("b", "a"),
        &["/b"],
        checksums.map(|c| c.b.as_str()),
    );
    fixture.write_package(
        "combined",
        &[&package_ref("a"), &package_ref("b")],
        "a_cpus=$(cat \"$SYSROOT/a\"); a_cpus=${a_cpus#-j}\nb_cpus=$(cat \"$SYSROOT/b\"); b_cpus=${b_cpus#-j}\ntest \"$((a_cpus + b_cpus))\" = 3\ninstall -Dm644 \"$SYSROOT/a\" \"$OUT_DIR/usr/share/combined\"\ninstall -Dm644 \"$SYSROOT/b\" \"$OUT_DIR/etc/existing\"",
        &["/usr/share/combined", "/etc/existing"],
        checksums.map(|c| c.combined.as_str()),
    );
    write_assemblies(
        fixture,
        checksums.map(|c| c.base.as_str()),
        checksums.map(|c| c.system.as_str()),
    );
}

fn write_assemblies(fixture: &Fixture, base_checksum: Option<&str>, checksum: Option<&str>) {
    let base_checksum = checksum_line(base_checksum);
    let base = format!(
        "system:\n  schema: 1\n  name: Base system\n  slug: base-system\n  version: 1.0\n  description: Graph test base\n{base_checksum}packages:\n- commit: {combined}\nfiles:\n- path: /etc/existing\n  content: |\n    replacement\n  replace: true\n- path: /var/lib/nex\n  directory: true\n  mode: 493\n- path: /combined-link\n  symlink: usr/share/combined\n- path: /etc/from-source\n  source: fixtures/config.txt\n  mode: 420\nbuild:\n  environment: {environment}\n  script: |\n    test \"$(cat \"$SYSROOT/target/etc/existing\")\" = replacement\n    test -L \"$SYSROOT/target/combined-link\"\n    test \"$(cat \"$SYSROOT/target/etc/from-source\")\" = 'from source'\n    install -Dm644 /dev/null \"$SYSROOT/target/base-ready\"\n",
        combined = package_ref("combined"),
        environment = fixture.environment,
    );
    fs::write(fixture.root.join("catalog/assemblies/base.yaml"), base)
        .expect("write base assembly");

    let checksum = checksum_line(checksum);
    let system = format!(
        "system:\n  schema: 1\n  name: Test system\n  slug: test-system\n  version: 1.0\n  description: Graph test system\n{checksum}base:\n  commit: systems/base-system/1.0\n  manifest: assemblies/base.yaml\nfiles:\n- path: /root-ready\n  content: ready\nbuild:\n  environment: {environment}\n  script: |\n    test -f \"$SYSROOT/target/base-ready\"\n    test \"$(cat \"$SYSROOT/target/root-ready\")\" = ready\n",
        environment = fixture.environment,
    );
    fs::write(fixture.root.join("catalog/assemblies/system.yaml"), system)
        .expect("write child assembly");
}

fn graph_checksums(fixture: &Fixture) -> Checksums {
    Checksums {
        a: fixture.checksum(&package_ref("a")),
        b: fixture.checksum(&package_ref("b")),
        combined: fixture.checksum(&package_ref("combined")),
        base: fixture.checksum("systems/base-system/1.0"),
        system: fixture.checksum("systems/test-system/1.0"),
    }
}

fn verify_system(fixture: &Fixture) {
    let checkout = fixture.root.join("checkout");
    zub::ops::checkout(
        &Repo::open(&fixture.repo).expect("open completed graph repo"),
        "systems/test-system/1.0",
        &checkout,
        CheckoutOptions::default(),
    )
    .expect("checkout completed system");
    assert_eq!(
        fs::read(checkout.join("etc/existing")).expect("read replaced assembly file"),
        b"replacement\n"
    );
    assert_eq!(
        fs::read(checkout.join("etc/from-source")).expect("read assembly source file"),
        b"from source\n"
    );
    assert!(checkout.join("var/lib/nex").is_dir());
    assert_eq!(
        fs::read_link(checkout.join("combined-link")).expect("read assembly symlink"),
        Path::new("usr/share/combined")
    );
    assert_eq!(
        fs::read(checkout.join("root-ready")).expect("read child assembly file"),
        b"ready"
    );
}

fn checksum_line(checksum: Option<&str>) -> String {
    checksum.map_or(String::new(), |value| format!("  checksum: {value}\n"))
}

fn package_ref(slug: &str) -> String {
    format!("x86_64/pkg/tests/{slug}/1.0/outputs/out")
}

fn marker_script(name: &str) -> &str {
    match name {
        "a" => "touch \"$SYNC/a\"\ninstall -Dm644 /dev/null \"$OUT_DIR/a\"",
        "b" => "touch \"$SYNC/b\"\ninstall -Dm644 /dev/null \"$OUT_DIR/b\"",
        "blocked" => "touch \"$SYNC/blocked\"\ninstall -Dm644 /dev/null \"$OUT_DIR/blocked\"",
        _ => unreachable!(),
    }
}

fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn init_git(root: &Path) {
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(root)
        .status()
        .expect("initialize graph fixture Git repo");
    assert!(status.success());
}

fn store_environment(root: &Path, sync: &Path) -> String {
    fs::write(
        root.join("environment.yaml"),
        format!(
            "name: test\nexecution:\n  chroot: false\npaths:\n  work: nex/work\n  out: nex/out\n  inputs: nex/inputs\nenv:\n  PATH: /usr/bin:/bin\n  SYSROOT: \"{{build_dir}}\"\n  WORK_DIR: \"{{build_dir}}/nex/work\"\n  OUT_DIR: \"{{build_dir}}/nex/out\"\n  SYNC: \"{}\"\n  MAKEFLAGS: \"-j{{num_cpus}}\"\npreamble: \"\"\n",
            sync.display()
        ),
    )
    .expect("write graph build environment");
    let output = Command::new("git")
        .args(["hash-object", "-w", "environment.yaml"])
        .current_dir(root)
        .output()
        .expect("store graph build environment");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("Git hash is UTF-8")
        .trim()
        .to_string()
}
