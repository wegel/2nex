//! Small Git helpers for repository-owned tasks.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

const GIT_LOCAL_ENV_VARS: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
];

pub(crate) fn repo_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    let output = git_output(&current_dir, &["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(output.trim()))
}

pub(crate) fn set_local_config(repo_root: &Path, key: &str, value: &str) -> Result<()> {
    run_git(repo_root, &["config", key, value])
}

pub(crate) fn staged_paths(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let output = git_output(
        repo_root,
        &["diff", "--cached", "--name-only", "--diff-filter=ACMR"],
    )?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect())
}

pub(crate) fn staged_rust_paths(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let output = git_output(
        repo_root,
        &[
            "diff",
            "--cached",
            "--name-only",
            "--diff-filter=ACMR",
            "--",
            "*.rs",
        ],
    )?;
    Ok(parse_paths(&output))
}

pub(crate) fn worktree_rust_paths(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let output = git_output(
        repo_root,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.rs",
        ],
    )?;
    Ok(parse_paths(&output))
}

pub(crate) fn staged_blob(repo_root: &Path, path: &Path) -> Result<String> {
    git_output(repo_root, &["show", &format!(":{}", path.display())])
}

fn parse_paths(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

pub(crate) fn run_git(repo_root: &Path, args: &[&str]) -> Result<()> {
    let status = command_for_repo(repo_root)
        .args(args)
        .status()
        .with_context(|| format!("run git {}", args.join(" ")))?;
    if status.success() {
        return Ok(());
    }
    bail!("git {} failed with status {status}", args.join(" "))
}

fn git_output(repo_root: &Path, args: &[&str]) -> Result<String> {
    let output = command_for_repo(repo_root)
        .args(args)
        .output()
        .with_context(|| format!("run git {}", args.join(" ")))?;
    if !output.status.success() {
        return Err(anyhow!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).context("decode git output")
}

pub(crate) fn command_for_repo(repo_root: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(repo_root);
    for variable in GIT_LOCAL_ENV_VARS {
        command.env_remove(variable);
    }
    command
}
