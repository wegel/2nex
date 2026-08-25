//! Flat system package materialization into the target root filesystem.

use std::fs;
use std::io;
use std::path::Path;

use std::collections::BTreeMap;

use crate::manifest::ManifestIndex;
use crate::materializer::checkout_files;
use crate::materializer::resolver::resolve_runtime_deps_precomputed;
use crate::materializer::types::MaterializeRequest;
use crate::store::checkout_into;

/// Materialize package commits into `/target` with a flat union checkout.
pub fn materialize_system_packages(
    base_dir: &str,
    repo_path: &str,
    package_commits: &[String],
    manifest_index: &ManifestIndex,
    providers: &BTreeMap<String, String>,
    preserve_target: bool,
) -> io::Result<()> {
    let target_dir = prepare_target_dir(base_dir, preserve_target)?;
    let requests = output_requests(package_commits);
    let closure =
        resolve_runtime_deps_precomputed(repo_path, &requests, manifest_index, providers, &[])?;

    if closure.has_unresolved() {
        return Err(unresolved_dependency_error(&closure));
    }

    for commit in closure.all_commits() {
        if let Some(files) = closure.get_files(commit) {
            let files_vec: Vec<String> = files.iter().cloned().collect();
            println!("  Checking out {} file(s) from {}", files_vec.len(), commit);
            checkout_files(repo_path, commit, &files_vec, &target_dir, &[])?;
        } else {
            checkout_into(repo_path, commit, &target_dir, true)?;
        }
    }

    Ok(())
}

pub(super) fn prepare_target_dir(base_dir: &str, preserve: bool) -> io::Result<std::path::PathBuf> {
    let target_dir = Path::new(base_dir).join("target");
    if target_dir.exists() && !preserve {
        fs::remove_dir_all(&target_dir)?;
    }
    fs::create_dir_all(&target_dir)?;
    Ok(target_dir)
}

fn output_requests(package_commits: &[String]) -> Vec<MaterializeRequest> {
    package_commits
        .iter()
        .map(|commit| MaterializeRequest::Output {
            commit: commit.clone(),
        })
        .collect()
}

fn unresolved_dependency_error(closure: &crate::materializer::types::RuntimeClosure) -> io::Error {
    let mut msg = String::from("Unresolved runtime dependencies:\n");
    for (dep, reasons) in &closure.unresolved {
        msg.push_str(&format!(
            "  {} - run 'nex compute-deps' on the package\n",
            dep
        ));
        for reason in reasons {
            msg.push_str(&format!("    needed by: {}\n", reason));
        }
    }
    io::Error::new(io::ErrorKind::NotFound, msg)
}
