//! Zub lookup and checkout for realized assembly bases.

use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::SystemBase;
use crate::store::{checkout_into_with_fallbacks, Store};
use crate::BuildOpts;

/// Resolve the moving base ref once so one child build uses one exact Zub commit.
pub(super) fn resolve_base_commit(
    base: Option<&SystemBase>,
    opts: &BuildOpts,
) -> io::Result<Option<String>> {
    let Some(base) = base else {
        return Ok(None);
    };
    let fallbacks = opts
        .fallback_repos
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let store = Store::open_with_fallback_chain(&opts.repo_path, &fallbacks)?;
    store.resolve_ref(&base.commit).map(Some).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("cannot resolve base system {}: {}", base.commit, error),
        )
    })
}

/// Layer an exact base commit into the child assembly target.
pub(super) fn layer_base_commit(
    commit: Option<&str>,
    opts: &BuildOpts,
    base_dir: &str,
) -> io::Result<()> {
    let Some(commit) = commit else {
        return Ok(());
    };
    checkout_into_with_fallbacks(
        &opts.repo_path,
        &opts.fallback_repos,
        commit,
        &Path::new(base_dir).join("target"),
        true,
        opts.verbose,
    )
}

#[cfg(test)]
#[path = "base_tests.rs"]
mod base_tests;
