//! JIT Materializer for 2nex packages.
//!
//! This module handles Just-In-Time resolution and checkout of packages
//! from OSTree. Instead of pre-computing runtime dependencies at build time,
//! dependencies are resolved by scanning ELF DT_NEEDED entries at materialization
//! time.
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
pub mod flatten;
pub mod index;
pub mod resolver;
pub mod types;

pub use checkout::{checkout_closure, checkout_files};
pub use flatten::flatten_capsule_precomputed;
pub use resolver::resolve_runtime_deps_precomputed;
pub use types::{
    MaterializeConfig, MaterializeMode, MaterializeRequest, MaterializeResult,
    RuntimeClosure,
};

use std::io;

use crate::manifest::ManifestIndex;

/// Main entry point for materializing packages.
///
/// This function:
/// 1. Loads ManifestIndex for precomputed dependency resolution
/// 2. Resolves runtime dependencies transitively using precomputed deps
/// 3. Checks out all required commits to the target directory
pub fn materialize(
    config: &MaterializeConfig,
    requests: &[MaterializeRequest],
) -> io::Result<MaterializeResult> {
    if requests.is_empty() {
        return Ok(MaterializeResult::new(RuntimeClosure::default()));
    }

    println!("Materializing {} request(s)...", requests.len());

    // collect initial commits
    let initial_commits: Vec<String> = requests.iter().map(|r| r.commit().to_string()).collect();

    // load manifest index (required for precomputed deps)
    let manifest_index = if let Some(ref db_path) = config.manifest_db_path {
        match ManifestIndex::load(db_path) {
            Ok(index) => {
                println!(
                    "  Loaded manifest index: {} manifests, {} files",
                    index.manifest_count(),
                    index.file_count()
                );
                index
            }
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Manifest index required for precomputed deps: {}", e),
                ));
            }
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "manifest_db_path is required for precomputed dependency resolution",
        ));
    };

    // resolve runtime dependencies using precomputed deps from manifests
    let closure = if config.resolve_deps {
        println!("  Resolving runtime dependencies (precomputed)...");
        resolve_runtime_deps_precomputed(&config.repo_path, requests, &manifest_index)?
    } else {
        // no resolution - just use the initial commits
        let mut closure = RuntimeClosure::default();
        for commit in &initial_commits {
            closure.add(commit, format!("requested: {}", commit));
        }
        closure
    };

    println!("  Runtime closure: {} commit(s)", closure.commits.len());

    // report unresolved dependencies
    if closure.has_unresolved() {
        println!("  Warning: {} unresolved dependencies:", closure.unresolved.len());
        for (req, reasons) in &closure.unresolved {
            println!("    - {}", req);
            for reason in reasons.iter().take(3) {
                println!("      {}", reason);
            }
        }
    }

    // checkout
    println!("  Checking out to {}...", config.target_dir.display());
    let result = checkout_closure(config, &closure)?;

    println!("Materialization complete.");
    Ok(result)
}

/// Convenience function to materialize a single bundle.
pub fn materialize_bundle(
    repo_path: &str,
    commit: &str,
    target_dir: &std::path::Path,
    mode: MaterializeMode,
) -> io::Result<MaterializeResult> {
    let config = MaterializeConfig {
        repo_path: repo_path.to_string(),
        target_dir: target_dir.to_path_buf(),
        mode,
        resolve_deps: true,
        ..Default::default()
    };

    let requests = vec![MaterializeRequest::Bundle {
        commit: commit.to_string(),
    }];

    materialize(&config, &requests)
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
mod tests {
    use super::*;

    #[test]
    fn test_materialize_config_default() {
        let config = MaterializeConfig::default();
        // repo_path is detected from environment, not hardcoded
        assert!(!config.repo_path.is_empty());
        assert_eq!(config.mode, MaterializeMode::Flat);
        assert!(config.resolve_deps);
    }

    #[test]
    fn test_materialize_request_commit() {
        let bundle = MaterializeRequest::Bundle {
            commit: "test/commit".to_string(),
        };
        assert_eq!(bundle.commit(), "test/commit");

        let output = MaterializeRequest::Output {
            commit: "test/output".to_string(),
        };
        assert_eq!(output.commit(), "test/output");

        let files = MaterializeRequest::Files {
            commit: "test/files".to_string(),
            paths: vec!["/bin/foo".to_string()],
        };
        assert_eq!(files.commit(), "test/files");
    }
}
