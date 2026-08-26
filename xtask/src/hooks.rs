//! Hook setup and hook-phase execution.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::cargo_project::{discover_projects, scopes_for_staged_paths, CargoScope, RustProject};
use crate::cli::CodeStyleArgs;
use crate::commit_message::{failure_message, validate_commit_message};
use crate::config::{
    load_hook_config, HookConfig, HookDefinition, HookPhase, HookPhaseConfig, PhaseMode,
    SourceScope,
};
use crate::{git, style};

pub(crate) fn install(repo_root: &Path) -> Result<()> {
    let hooks_dir = repo_root.join(".githooks");
    if !hooks_dir.is_dir() {
        bail!("tracked hook directory missing at {}", hooks_dir.display());
    }
    git::set_local_config(repo_root, "core.hooksPath", ".githooks")?;
    println!("hooks installed at .githooks");
    Ok(())
}

pub(crate) fn run_pre_commit(repo_root: &Path) -> Result<()> {
    let config = load_hook_config(repo_root)?;
    let projects = discover_projects(repo_root, &config.rust_projects)?;
    let staged_paths = git::staged_paths(repo_root)?;
    let scopes = scopes_for_staged_paths(&projects, &staged_paths);
    run_hook(
        repo_root,
        &config,
        &config.hooks.pre_commit,
        &projects,
        &scopes,
        None,
    )
}

pub(crate) fn run_commit_msg(repo_root: &Path, message_file: &Path) -> Result<()> {
    let config = load_hook_config(repo_root)?;
    run_hook(
        repo_root,
        &config,
        &config.hooks.commit_msg,
        &[],
        &BTreeMap::new(),
        Some(message_file),
    )
}

pub(crate) fn run_pre_push(repo_root: &Path) -> Result<()> {
    let config = load_hook_config(repo_root)?;
    run_hook(
        repo_root,
        &config,
        &config.hooks.pre_push,
        &[],
        &BTreeMap::new(),
        None,
    )
}

pub(crate) fn run_code_style(repo_root: &Path, args: CodeStyleArgs) -> Result<()> {
    if !args.staged && !args.all {
        bail!("code-style requires either --staged or --all");
    }
    let config = load_hook_config(repo_root)?;
    let projects = discover_projects(repo_root, &config.rust_projects)?;
    let violations = if args.staged {
        style::check_staged_rust_files(repo_root, &config.code_style, &projects)?
    } else {
        style::check_all_rust_files(repo_root, &config.code_style, &projects)?
    };
    report_style_violations(violations)
}

fn run_hook(
    repo_root: &Path,
    config: &HookConfig,
    hook: &HookDefinition,
    projects: &[RustProject],
    scopes: &BTreeMap<usize, CargoScope>,
    message_file: Option<&Path>,
) -> Result<()> {
    for phase in &hook.phases {
        if let Err(error) = run_phase(repo_root, config, phase, projects, scopes, message_file) {
            match phase.mode {
                PhaseMode::Blocking => return Err(error),
                PhaseMode::Advisory => eprintln!("warning: {error:#}"),
            }
        }
    }
    Ok(())
}

fn run_phase(
    repo_root: &Path,
    config: &HookConfig,
    phase: &HookPhaseConfig,
    projects: &[RustProject],
    scopes: &BTreeMap<usize, CargoScope>,
    message_file: Option<&Path>,
) -> Result<()> {
    match phase.kind {
        HookPhase::CodeStyle => {
            let scope = phase.scope.unwrap_or(SourceScope::Staged);
            let violations = if scope == SourceScope::Staged {
                style::check_staged_rust_files(repo_root, &config.code_style, projects)?
            } else {
                style::check_all_rust_files(repo_root, &config.code_style, projects)?
            };
            report_style_violations(violations)
        }
        HookPhase::Fmt | HookPhase::Clippy | HookPhase::Test => {
            run_cargo_phase(repo_root, phase.kind, projects, scopes)
        }
        HookPhase::CommitMessage => run_commit_message_phase(message_file),
    }
}

fn run_cargo_phase(
    repo_root: &Path,
    phase: HookPhase,
    projects: &[RustProject],
    scopes: &BTreeMap<usize, CargoScope>,
) -> Result<()> {
    let label = match phase {
        HookPhase::CodeStyle => unreachable!("source style does not run Cargo"),
        HookPhase::Fmt => "cargo fmt",
        HookPhase::Clippy => "cargo clippy",
        HookPhase::Test => "cargo test",
        HookPhase::CommitMessage => unreachable!("commit messages do not run Cargo"),
    };
    if scopes.is_empty() {
        eprintln!("{label}: no staged Rust source or manifest, skipping");
        return Ok(());
    }

    for (index, scope) in scopes {
        let project = projects
            .get(*index)
            .context("Cargo scope points to a missing project")?;
        let args = cargo_args(phase, project, scope);
        let scope_label = match scope {
            CargoScope::Workspace => "workspace".to_owned(),
            CargoScope::Packages(packages) => {
                packages.iter().cloned().collect::<Vec<_>>().join(", ")
            }
        };
        eprintln!("{label}: {} ({scope_label})", project.name);

        let status = Command::new("cargo")
            .args(&args)
            .current_dir(repo_root)
            .status()
            .with_context(|| format!("run {label} for {}", project.name))?;
        if !status.success() {
            bail!("{label} failed for {} with status {status}", project.name);
        }
    }
    Ok(())
}

fn report_style_violations(violations: Vec<style::StyleViolation>) -> Result<()> {
    if violations.is_empty() {
        return Ok(());
    }
    for violation in violations {
        eprintln!("{violation}");
    }
    bail!("code-style found violations")
}

fn cargo_args(phase: HookPhase, project: &RustProject, scope: &CargoScope) -> Vec<String> {
    let mut args = vec![
        match phase {
            HookPhase::CodeStyle => unreachable!("source style does not run Cargo"),
            HookPhase::Fmt => "fmt",
            HookPhase::Clippy => "clippy",
            HookPhase::Test => "test",
            HookPhase::CommitMessage => unreachable!("commit messages do not run Cargo"),
        }
        .to_owned(),
        "--manifest-path".to_owned(),
        project.manifest_path.display().to_string(),
    ];
    match scope {
        CargoScope::Workspace => args.push(match phase {
            HookPhase::CodeStyle => unreachable!("source style does not run Cargo"),
            HookPhase::Fmt => "--all".to_owned(),
            HookPhase::Clippy | HookPhase::Test => "--workspace".to_owned(),
            HookPhase::CommitMessage => unreachable!("commit messages do not run Cargo"),
        }),
        CargoScope::Packages(packages) => {
            for package in packages {
                args.extend(["--package".to_owned(), package.clone()]);
            }
        }
    }
    match phase {
        HookPhase::CodeStyle => unreachable!("source style does not run Cargo"),
        HookPhase::Fmt => args.extend(["--".to_owned(), "--check".to_owned()]),
        HookPhase::Clippy => {
            args.extend(project.clippy_args.iter().cloned());
            args.extend(["--".to_owned(), "-D".to_owned(), "warnings".to_owned()]);
        }
        HookPhase::Test => args.extend(project.test_args.iter().cloned()),
        HookPhase::CommitMessage => unreachable!("commit messages do not run Cargo"),
    }
    args
}

fn run_commit_message_phase(message_file: Option<&Path>) -> Result<()> {
    let path = message_file.context("commit-message phase requires a message file")?;
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    validate_commit_message(&raw).map_err(|_| anyhow::anyhow!(failure_message()))
}

#[cfg(test)]
#[path = "hooks_tests.rs"]
mod tests;
