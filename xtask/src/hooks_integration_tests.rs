//! Repository-fixture tests for the complete hook entrypoints.

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use tempfile::TempDir;

use crate::{git, hooks};

#[test]
fn pre_commit_checks_a_staged_rust_project() -> Result<()> {
    let repository = TempDir::new()?;
    init_repository(repository.path())?;
    write_file(
        repository.path(),
        ".githooks/config.toml",
        r#"
            [hooks.pre_commit]
            phases = [
              { kind = "code-style", mode = "blocking", scope = "staged" },
              { kind = "fmt", mode = "blocking" },
              { kind = "clippy", mode = "blocking" },
              { kind = "test", mode = "blocking" },
            ]

            [[rust_projects]]
            name = "demo"
            manifest_path = "demo/Cargo.toml"
        "#,
    )?;
    write_file(
        repository.path(),
        "demo/Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )?;
    write_file(
        repository.path(),
        "demo/src/lib.rs",
        "//! Small fixture checked by every pre-commit phase.\n\npub fn answer() -> u8 {\n    42\n}\n",
    )?;
    run_git(repository.path(), &["add", "."])?;

    hooks::run_pre_commit(repository.path())
}

#[test]
fn install_points_git_at_the_tracked_hooks() -> Result<()> {
    let repository = TempDir::new()?;
    init_repository(repository.path())?;
    fs::create_dir(repository.path().join(".githooks"))?;

    hooks::install(repository.path())?;

    let output = git::command_for_repo(repository.path())
        .args(["config", "--get", "core.hooksPath"])
        .output()?;
    if !output.status.success() {
        bail!("git did not return core.hooksPath");
    }
    assert_eq!(String::from_utf8(output.stdout)?.trim(), ".githooks");
    Ok(())
}

fn init_repository(path: &Path) -> Result<()> {
    run_git(path, &["init", "--quiet"])
}

fn write_file(root: &Path, relative: &str, contents: &str) -> Result<()> {
    let path = root.join(relative);
    let parent = path
        .parent()
        .with_context(|| format!("find parent for {}", path.display()))?;
    fs::create_dir_all(parent)?;
    fs::write(path, contents)?;
    Ok(())
}

fn run_git(root: &Path, arguments: &[&str]) -> Result<()> {
    let status = git::command_for_repo(root).args(arguments).status()?;
    if !status.success() {
        bail!("git {} failed with status {status}", arguments.join(" "));
    }
    Ok(())
}
