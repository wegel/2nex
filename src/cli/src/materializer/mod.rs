//! Materialize package closures from the store into root filesystems.
//!
//! This module resolves runtime dependencies from precomputed manifest metadata
//! and checks out the required store refs. It does not scan ELF files while
//! materializing a root.
//!
//! # Usage
//!
//! ```ignore
//! use nex::materializer::{materialize, MaterializeRequest, MaterializeConfig, MaterializeMode};
//!
//! let config = MaterializeConfig {
//!     repo_path: "bootstrap_store".to_string(),
//!     target_dir: PathBuf::from("/mnt/rootfs"),
//!     mode: MaterializeMode::Flat,
//!     ..Default::default()
//! };
//!
//! let requests = vec![
//!     MaterializeRequest::Bundle { commit: "x86_64/pkg/base/glibc/2.40/bundles/dev".to_string() },
//! ];
//!
//! let result = materialize(&config, &requests)?;
//! ```

pub mod checkout;
mod checkout_refs;
mod checkout_store;
pub mod flatten;
mod flatten_deps;
mod flatten_errors;
pub mod index;
mod pathdiff;
pub mod resolver;
mod resolver_entries;
mod resolver_metadata;
mod resolver_refs;
pub mod types;

pub use checkout::checkout_closure;
pub use checkout_store::checkout_files;
pub use flatten::flatten_capsule_precomputed;
pub use resolver::resolve_runtime_deps_precomputed;
pub use types::{
    MaterializeConfig, MaterializeMode, MaterializeRequest, MaterializeResult, RuntimeClosure,
};

use std::io;

use crate::manifest::ManifestIndex;

/// Materialize the requested package refs into the configured target directory.
pub fn materialize(
    config: &MaterializeConfig,
    requests: &[MaterializeRequest],
) -> io::Result<MaterializeResult> {
    if requests.is_empty() {
        return Ok(MaterializeResult::new(RuntimeClosure::default()));
    }

    println!("Materializing {} request(s)...", requests.len());

    let closure = runtime_closure(config, requests)?;
    reject_unresolved_dependencies(&closure)?;

    println!("  Checking out to {}...", config.target_dir.display());
    let result = checkout_closure(config, &closure)?;

    println!("Materialization complete.");
    Ok(result)
}

fn load_materializer_index(config: &MaterializeConfig) -> io::Result<ManifestIndex> {
    if config.manifest_db_paths.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "manifest_db_paths is required for precomputed dependency resolution",
        ));
    }

    let index = ManifestIndex::load_layered(&config.manifest_db_paths).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Manifest index required for precomputed deps: {}", e),
        )
    })?;
    println!(
        "  Loaded manifest index: {} manifests, {} files",
        index.manifest_count(),
        index.file_count()
    );
    Ok(index)
}

fn runtime_closure(
    config: &MaterializeConfig,
    requests: &[MaterializeRequest],
) -> io::Result<RuntimeClosure> {
    let closure = if config.resolve_deps {
        println!("  Resolving runtime dependencies (precomputed)...");
        let manifest_index = load_materializer_index(config)?;
        resolve_runtime_deps_precomputed(
            &config.repo_path,
            requests,
            &manifest_index,
            &config.fallback_repo_paths,
        )?
    } else {
        requested_only_closure(requests)
    };

    println!("  Runtime closure: {} commit(s)", closure.commits.len());
    Ok(closure)
}

fn requested_only_closure(requests: &[MaterializeRequest]) -> RuntimeClosure {
    let mut closure = RuntimeClosure::default();
    for commit in requests.iter().map(|request| request.commit()) {
        closure.add(commit, format!("requested: {}", commit));
    }
    closure
}

fn reject_unresolved_dependencies(closure: &RuntimeClosure) -> io::Result<()> {
    if !closure.has_unresolved() {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        unresolved_dependencies_message(closure),
    ))
}

fn unresolved_dependencies_message(closure: &RuntimeClosure) -> String {
    let mut message = format!(
        "{} unresolved runtime dependency requirement(s):",
        closure.unresolved.len()
    );
    for (requirement, reasons) in &closure.unresolved {
        message.push_str(&format!("\n  {}", requirement));
        for reason in reasons.iter().take(3) {
            message.push_str(&format!("\n    needed by: {}", reason));
        }
    }
    message
}

/// Convenience function to materialize a single bundle.
pub fn materialize_bundle(
    repo_path: &str,
    commit: &str,
    target_dir: &std::path::Path,
    mode: MaterializeMode,
) -> io::Result<MaterializeResult> {
    let config = bundle_materialize_config(repo_path, target_dir, mode);

    let requests = vec![MaterializeRequest::Bundle {
        commit: commit.to_string(),
    }];

    materialize(&config, &requests)
}

fn bundle_materialize_config(
    repo_path: &str,
    target_dir: &std::path::Path,
    mode: MaterializeMode,
) -> MaterializeConfig {
    MaterializeConfig {
        repo_path: repo_path.to_string(),
        target_dir: target_dir.to_path_buf(),
        mode,
        resolve_deps: false,
        ..Default::default()
    }
}

/// Convenience function to materialize multiple outputs (flat union).
pub fn materialize_outputs_flat(
    repo_path: &str,
    commits: &[String],
    target_dir: &std::path::Path,
) -> io::Result<MaterializeResult> {
    let config = MaterializeConfig {
        repo_path: repo_path.to_string(),
        target_dir: target_dir.to_path_buf(),
        mode: MaterializeMode::Flat,
        resolve_deps: false, // typically outputs are already part of a closure
        ..Default::default()
    };

    let requests: Vec<MaterializeRequest> = commits
        .iter()
        .map(|c| MaterializeRequest::Output {
            commit: c.to_string(),
        })
        .collect();

    materialize(&config, &requests)
}

#[cfg(test)]
#[path = "materializer_tests.rs"]
mod materializer_tests;
