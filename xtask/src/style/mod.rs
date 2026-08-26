//! Rust source rules for staged blobs and tracked files.

mod rust;
mod sources;

use std::fmt;
use std::path::PathBuf;

use anyhow::Result;

use crate::cargo_project::RustProject;
use crate::config::CodeStyleConfig;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StyleViolation {
    pub(crate) rule_id: &'static str,
    pub(crate) path: PathBuf,
    pub(crate) line: Option<usize>,
    pub(crate) message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CrateKind {
    Library,
    Binary,
}

pub(crate) fn check_staged_rust_files(
    repo_root: &std::path::Path,
    config: &CodeStyleConfig,
    projects: &[RustProject],
) -> Result<Vec<StyleViolation>> {
    let sources = sources::staged_rust_sources(repo_root, config, projects)?;
    Ok(check_sources(&sources))
}

pub(crate) fn check_all_rust_files(
    repo_root: &std::path::Path,
    config: &CodeStyleConfig,
    projects: &[RustProject],
) -> Result<Vec<StyleViolation>> {
    let sources = sources::all_rust_sources(repo_root, config, projects)?;
    Ok(check_sources(&sources))
}

fn check_sources(sources: &[sources::StyleSource]) -> Vec<StyleViolation> {
    sources.iter().flat_map(rust::check_rust_source).collect()
}

impl fmt::Display for StyleViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(line) = self.line {
            return write!(
                formatter,
                "{} {}:{} {}",
                self.rule_id,
                self.path.display(),
                line,
                self.message
            );
        }
        write!(
            formatter,
            "{} {} {}",
            self.rule_id,
            self.path.display(),
            self.message
        )
    }
}

#[cfg(test)]
mod tests;
