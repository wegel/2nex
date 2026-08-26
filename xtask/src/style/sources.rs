//! Read eligible Rust source from the Git index or worktree.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::cargo_project::{is_library_source, RustProject};
use crate::config::CodeStyleConfig;
use crate::git;
use crate::style::CrateKind;

#[derive(Clone, Debug)]
pub(crate) struct StyleSource {
    pub(crate) path: PathBuf,
    pub(crate) crate_kind: CrateKind,
    pub(crate) source: String,
}

pub(crate) fn staged_rust_sources(
    repo_root: &Path,
    config: &CodeStyleConfig,
    projects: &[RustProject],
) -> Result<Vec<StyleSource>> {
    git::staged_rust_paths(repo_root)?
        .into_iter()
        .filter(|path| is_eligible(path, config))
        .map(|path| {
            let source = git::staged_blob(repo_root, &path)
                .with_context(|| format!("read staged blob {}", path.display()))?;
            Ok(StyleSource {
                crate_kind: crate_kind(projects, &path),
                path,
                source,
            })
        })
        .collect()
}

pub(crate) fn all_rust_sources(
    repo_root: &Path,
    config: &CodeStyleConfig,
    projects: &[RustProject],
) -> Result<Vec<StyleSource>> {
    git::worktree_rust_paths(repo_root)?
        .into_iter()
        .filter(|path| is_eligible(path, config))
        .map(|path| {
            let source = fs::read_to_string(repo_root.join(&path))
                .with_context(|| format!("read {}", path.display()))?;
            Ok(StyleSource {
                crate_kind: crate_kind(projects, &path),
                path,
                source,
            })
        })
        .collect()
}

fn is_eligible(path: &Path, config: &CodeStyleConfig) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("rs")
        && !config
            .excluded_paths
            .iter()
            .any(|excluded| path.starts_with(excluded))
}

fn crate_kind(projects: &[RustProject], path: &Path) -> CrateKind {
    if is_library_source(projects, path) {
        CrateKind::Library
    } else {
        CrateKind::Binary
    }
}
