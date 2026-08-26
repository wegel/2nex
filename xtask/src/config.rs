//! Data loaded from `.githooks/config.toml`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct HookConfig {
    pub(crate) hooks: HookSections,
    #[serde(default)]
    pub(crate) code_style: CodeStyleConfig,
    pub(crate) rust_projects: Vec<RustProjectConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct HookSections {
    #[serde(default)]
    pub(crate) pre_commit: HookDefinition,
    #[serde(default)]
    pub(crate) commit_msg: HookDefinition,
    #[serde(default)]
    pub(crate) pre_push: HookDefinition,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct HookDefinition {
    #[serde(default)]
    pub(crate) phases: Vec<HookPhaseConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct HookPhaseConfig {
    pub(crate) kind: HookPhase,
    #[serde(default)]
    pub(crate) mode: PhaseMode,
    #[serde(default)]
    pub(crate) scope: Option<SourceScope>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HookPhase {
    CodeStyle,
    Fmt,
    Clippy,
    Test,
    CommitMessage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SourceScope {
    Staged,
    All,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PhaseMode {
    #[default]
    Blocking,
    Advisory,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RustProjectConfig {
    pub(crate) name: String,
    pub(crate) manifest_path: PathBuf,
    #[serde(default)]
    pub(crate) clippy_args: Vec<String>,
    #[serde(default)]
    pub(crate) test_args: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct CodeStyleConfig {
    #[serde(default)]
    pub(crate) excluded_paths: Vec<PathBuf>,
}

pub(crate) fn load_hook_config(repo_root: &Path) -> Result<HookConfig> {
    let path = repo_root.join(".githooks/config.toml");
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let config: HookConfig =
        toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    validate_config(repo_root, &config)?;
    Ok(config)
}

fn validate_config(repo_root: &Path, config: &HookConfig) -> Result<()> {
    if config.rust_projects.is_empty() {
        bail!("hook config must name at least one Rust project");
    }

    let mut names = BTreeSet::new();
    let mut manifests = BTreeSet::new();
    for project in &config.rust_projects {
        if project.name.trim().is_empty() {
            bail!("Rust project name must not be empty");
        }
        if !names.insert(&project.name) {
            bail!(
                "Rust project name {:?} appears more than once",
                project.name
            );
        }
        validate_relative_path(&project.manifest_path)?;
        if !manifests.insert(&project.manifest_path) {
            bail!(
                "Rust manifest {} appears more than once",
                project.manifest_path.display()
            );
        }

        let manifest = repo_root.join(&project.manifest_path);
        if !manifest.is_file() {
            bail!("Rust manifest does not exist: {}", manifest.display());
        }
    }
    for path in &config.code_style.excluded_paths {
        validate_relative_path(path)?;
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        bail!("hook paths must be non-empty paths below the repository root");
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("hook path must not contain '.' or '..': {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
