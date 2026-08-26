//! Execution, reuse checks, and logs for one graph node.

use std::fs;
use std::io;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;

use zub::Repo;

use crate::assembly::run_assembly;
use crate::builder::{run_package, BuildContext};
use crate::graph::{Node, NodeKind};
use crate::metadata;
use crate::output::published_refs;
use crate::profile::update_manifest;
use crate::progress::Reporter;
use crate::recipe;
use crate::sandbox::JobOutput;
use crate::{Error, Result};

// --- Executor state ---

#[derive(Clone)]
pub(crate) struct Executor {
    pub repo: PathBuf,
    pub build_dir: PathBuf,
    pub source_cache: PathBuf,
    pub check: bool,
    pub adopt_existing: bool,
    pub cpu_count: NonZeroUsize,
    pub reporter: Arc<Reporter>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum NodeOutcome {
    Built,
    Reused,
}

// --- Node execution ---

pub(crate) fn execute_node(node: &Node, index: usize, executor: &Executor) -> Result<NodeOutcome> {
    let repo = Repo::open(&executor.repo)?;
    let recipe = recipe::compute(&repo, node)?;
    if !executor.reporter.options().record_profile
        && reuse_if_current(&repo, node, &recipe, executor.adopt_existing)?
    {
        executor.reporter.reused(&node.label);
        return Ok(NodeOutcome::Reused);
    }

    let root = executor.build_dir.join("nodes").join(format!("{index:05}"));
    let log = executor
        .build_dir
        .join("logs")
        .join(log_filename(index, &node.label));
    fs::create_dir_all(log.parent().expect("log parent"))?;
    fs::File::create(&log)?;
    let progress = executor
        .reporter
        .start(index, &node.label, node_profile(node), executor.check);
    let output = JobOutput::new(log.clone(), Arc::clone(&progress));
    let result = build_node(&repo, node, executor, &root, &output, &recipe)
        .and_then(|_| save_profile(executor, node, &output));

    match result {
        Ok(()) => {
            executor.reporter.finished(index, &progress);
            Ok(NodeOutcome::Built)
        }
        Err(error) => {
            executor.reporter.failed(index, &progress);
            Err(Error::NodeFailed {
                label: node.label.clone(),
                log,
                source: Box::new(error),
            })
        }
    }
}

fn build_node(
    repo: &Repo,
    node: &Node,
    executor: &Executor,
    root: &std::path::Path,
    output: &JobOutput,
    recipe: &str,
) -> Result<()> {
    let context = BuildContext {
        repo,
        environment: &node.environment,
        manifest_path: &node.manifest_path,
        source_cache: &executor.source_cache,
        build_root: root,
        check: executor.check,
        cpu_count: executor.cpu_count,
        recipe,
        output: Some(output),
    };
    match &node.kind {
        NodeKind::Package { manifest, inputs } => run_package(manifest, inputs, &context),
        NodeKind::Assembly {
            manifest,
            build_inputs,
            packages,
        } => run_assembly(manifest, build_inputs, packages, &context),
    }
}

fn save_profile(executor: &Executor, node: &Node, output: &JobOutput) -> Result<()> {
    if !executor.reporter.options().record_profile {
        return Ok(());
    }
    let profile = output
        .recorded_profile()
        .ok_or_else(|| io::Error::other("build produced no timing profile"))?;
    Ok(update_manifest(&node.manifest_path, &profile)?)
}

// --- Reuse checks and result refs ---

fn reuse_if_current(
    repo: &Repo,
    node: &Node,
    recipe: &str,
    adopt_existing: bool,
) -> io::Result<bool> {
    match &node.kind {
        NodeKind::Package { manifest, .. } => reuse_package(repo, manifest, recipe, adopt_existing),
        NodeKind::Assembly { manifest, .. } => reuse_commits(
            repo,
            &[manifest.reference()],
            manifest.system.checksum.as_deref(),
            recipe,
            adopt_existing,
        ),
    }
}

fn reuse_package(
    repo: &Repo,
    manifest: &crate::manifest::PackageManifest,
    recipe: &str,
    adopt_existing: bool,
) -> io::Result<bool> {
    let refs = published_refs(manifest);
    reuse_commits(
        repo,
        &refs,
        manifest.package.checksum.as_deref(),
        recipe,
        adopt_existing,
    )
}

fn reuse_commits(
    repo: &Repo,
    references: &[String],
    expected: Option<&str>,
    recipe: &str,
    adopt_existing: bool,
) -> io::Result<bool> {
    let mut stored = Vec::with_capacity(references.len());
    let mut common_checksum: Option<String> = None;
    for reference in references {
        let Ok(hash) = zub::resolve_ref(repo, reference) else {
            return Ok(false);
        };
        let commit = zub::read_commit(repo, &hash).map_err(io::Error::other)?;
        if let Some(checksum) = commit.metadata.get(metadata::CHECKSUM).map(String::as_str) {
            if expected.is_some_and(|value| value != checksum)
                || common_checksum
                    .as_deref()
                    .is_some_and(|value| value != checksum)
            {
                return Ok(false);
            }
            common_checksum.get_or_insert_with(|| checksum.to_owned());
        } else if !adopt_existing {
            return Ok(false);
        }
        stored.push((reference, commit));
    }
    if let Some(expected) = expected {
        if stored.iter().all(|(_, commit)| {
            commit.metadata.get(metadata::CHECKSUM).map(String::as_str) == Some(expected)
        }) {
            return Ok(true);
        }
    }
    if stored.iter().all(|(_, commit)| has_recipe(commit, recipe)) {
        return Ok(true);
    }
    if stored
        .iter()
        .any(|(_, commit)| commit.metadata.contains_key(metadata::RECIPE))
        || !adopt_existing
    {
        return Ok(false);
    }
    let Some(checksum) = common_checksum.or_else(|| expected.map(str::to_owned)) else {
        return Ok(false);
    };
    adopt_commits(repo, stored, &checksum, recipe)?;
    Ok(true)
}

fn adopt_commits(
    repo: &Repo,
    stored: Vec<(&String, zub::Commit)>,
    checksum: &str,
    recipe: &str,
) -> io::Result<()> {
    for (reference, commit) in stored {
        let metadata = metadata::build(checksum, recipe);
        zub::ops::commit_tree_with_metadata(
            repo,
            &commit.tree,
            reference,
            Some(""),
            Some(metadata::AUTHOR),
            &metadata,
        )
        .map_err(io::Error::other)?;
    }
    Ok(())
}

fn has_recipe(commit: &zub::Commit, recipe: &str) -> bool {
    commit.metadata.contains_key(metadata::CHECKSUM)
        && commit
            .metadata
            .get(metadata::RECIPE)
            .is_some_and(|value| value == recipe)
}

pub(crate) fn node_references(node: &Node) -> Vec<String> {
    match &node.kind {
        NodeKind::Package { manifest, .. } => published_refs(manifest),
        NodeKind::Assembly { manifest, .. } => vec![manifest.reference()],
    }
}

fn node_profile(node: &Node) -> &[String] {
    match &node.kind {
        NodeKind::Package { manifest, .. } => &manifest.build.profile,
        NodeKind::Assembly { manifest, .. } => &manifest.build.profile,
    }
}

fn log_filename(index: usize, label: &str) -> String {
    let mut safe = String::with_capacity(label.len());
    let mut separator = false;
    for character in label.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
            safe.push(character);
            separator = false;
        } else if !separator {
            safe.push('-');
            separator = true;
        }
    }
    format!("{index:05}-{}.log", safe.trim_matches('-'))
}

#[cfg(test)]
#[path = "node_runner_tests.rs"]
mod tests;
