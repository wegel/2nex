//! CLI tests for the conventional product plus upstream/nex repository layout.

use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn check_loads_product_and_upstream_without_a_workspace_file() -> io::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let product = temp_dir.path().join("edge-product");
    let upstream = product.join("upstream/nex");
    initialize_repository(&product)?;
    initialize_repository(&upstream)?;
    write_fixture(&product, &upstream)?;
    assert!(!product.join("nex-workspace.yaml").exists());

    let output = run_check(&product)?;

    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("all checks passed"));
    let build_output = run_dry_build(&product)?;
    assert_success(&build_output);
    let build_stdout = String::from_utf8_lossy(&build_output.stdout);
    assert!(build_stdout.contains("runtime"), "{build_stdout}");
    assert!(build_stdout.contains("product-agent"), "{build_stdout}");
    Ok(())
}

#[test]
fn check_rejects_a_product_manifest_that_shadows_upstream() -> io::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let product = temp_dir.path().join("edge-product");
    let upstream = product.join("upstream/nex");
    initialize_repository(&product)?;
    initialize_repository(&upstream)?;
    write_fixture(&product, &upstream)?;
    let duplicate = product.join("pkg/libs/runtime-copy.yaml");
    fs::create_dir_all(duplicate.parent().expect("duplicate parent"))?;
    fs::write(&duplicate, runtime_manifest())?;

    let output = run_check(&product)?;

    assert!(!output.status.success(), "duplicate package should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("duplicate package manifest identity"),
        "{stderr}"
    );
    assert!(stderr.contains("product/pkg"), "{stderr}");
    assert!(stderr.contains("upstream/nex/pkg"), "{stderr}");
    Ok(())
}

fn initialize_repository(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    let output = Command::new("git").arg("init").arg(path).output()?;
    if !output.status.success() {
        return Err(io::Error::other(String::from_utf8_lossy(&output.stderr)));
    }
    git(path, &["config", "user.email", "test@example.test"])?;
    git(path, &["config", "user.name", "Test"])?;
    Ok(())
}

fn write_fixture(product: &Path, upstream: &Path) -> io::Result<()> {
    write(upstream, "pkg/libs/runtime.yaml", runtime_manifest())?;
    git(upstream, &["add", "pkg/libs/runtime.yaml"])?;
    git(upstream, &["commit", "-m", "add runtime"])?;
    let runtime_revision = git(upstream, &["rev-parse", "HEAD"])?;
    write(upstream, "asm/base.yaml", base_assembly())?;
    write(
        upstream,
        "asm/base-overlay.yaml",
        overlay("/etc/upstream-base"),
    )?;
    write(
        product,
        "pkg/product/product-agent.yaml",
        agent_manifest(&runtime_revision),
    )?;
    write(product, "asm/device.yaml", product_assembly())?;
    write(
        product,
        "asm/device-overlay.yaml",
        overlay("/etc/product-device"),
    )?;
    format_product_assembly(product)
}

fn git(directory: &Path, args: &[&str]) -> io::Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn format_product_assembly(product: &Path) -> io::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_nex"))
        .args(["format", "asm/device.yaml"])
        .current_dir(product)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(String::from_utf8_lossy(&output.stderr)))
    }
}

fn write(root: &Path, relative_path: &str, content: impl AsRef<[u8]>) -> io::Result<()> {
    let path = root.join(relative_path);
    fs::create_dir_all(path.parent().expect("fixture parent"))?;
    fs::write(path, content)
}

fn run_check(product: &Path) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_nex"))
        .args(["check", "asm/device.yaml"])
        .current_dir(product)
        .output()
}

fn run_dry_build(product: &Path) -> io::Result<Output> {
    let store = product.join("empty-store");
    fs::create_dir_all(&store)?;
    Command::new(env!("CARGO_BIN_EXE_nex"))
        .args([
            "build",
            "asm/device.yaml",
            "--repo",
            "empty-store",
            "--dry-run",
            "--force",
        ])
        .current_dir(product)
        .output()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn runtime_manifest() -> &'static str {
    r#"package:
  schema: 1
  name: Runtime
  slug: runtime
  namespace: libs
  version: 1.0

sources: []

dependencies: []

build:
  environment: abcdef
  script: "true"

bundles:
  full:
  - lib

outputs:
  lib:
    files:
    - path: /usr/lib/libruntime.so.1
"#
}

fn agent_manifest(runtime_revision: &str) -> String {
    format!(
        r#"package:
  schema: 1
  name: Product Agent
  slug: product-agent
  namespace: product
  version: 1.0

sources: []

dependencies:
- name: runtime
  commit: x86_64/pkg/libs/runtime/1.0/outputs/lib
  manifest_ref: {runtime_revision}

build:
  environment: abcdef
  script: "true"

bundles:
  full:
  - bin

outputs:
  bin:
    files:
    - path: /usr/bin/product-agent
      needs:
      - /usr/lib/libruntime.so.1

resolution:
  /usr/lib/libruntime.so.1: runtime
"#
    )
}

fn base_assembly() -> &'static str {
    r#"system:
  schema: 1
  name: Upstream Base
  slug: upstream-base
  version: 1.0

overlays:
- asm/base-overlay.yaml

packages: []

build:
  environment: abcdef
  script: ""
"#
}

fn product_assembly() -> &'static str {
    r#"system:
  schema: 1
  name: Product Device
  slug: product-device
  version: 1.0
  extends: upstream/nex/asm/base.yaml

overlays:
- asm/device-overlay.yaml

packages:
- name: product-agent
  commit: x86_64/pkg/product/product-agent/1.0/outputs/bin

build:
  environment: abcdef
  script: ""
"#
}

fn overlay(path: &str) -> String {
    format!("files:\n- path: {path}\n  mode: 420\n  content: |\n    fixture\n")
}
