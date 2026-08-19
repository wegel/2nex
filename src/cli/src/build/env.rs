//! Build environment loading and template expansion.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manifest::repositories::repository_root_for_path;
use crate::manifest::types::BuildEnvironment;

fn is_sha1(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// Load a build environment from a Git blob SHA1 or filesystem path.
///
/// A blob is read from the Git repository that owns `manifest_path`, because
/// that is where the manifest's history lives. Resolving against the package
/// store instead only appears to work in a development checkout, where the
/// store happens to sit inside the source repository; on an installed machine
/// the store is a sibling of the manifests repository and holds no Git objects
/// at all.
pub fn load_environment(
    manifest_path: &Path,
    fallback_path: &str,
    env_ref: &str,
) -> io::Result<BuildEnvironment> {
    let content = if is_sha1(env_ref) {
        let owning_root = environment_repository_root(manifest_path, fallback_path);
        load_environment_blob(&owning_root.to_string_lossy(), env_ref)?
    } else {
        load_environment_file(env_ref)?
    };

    serde_yaml::from_str(&content).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse environment YAML: {}", err),
        )
    })
}

/// Expand build environment template variables in a manifest string value.
pub fn expand_env_templates(
    value: &str,
    num_cpus: usize,
    build_dir: &str,
    bootstrap_tools: Option<&str>,
    bootstrap_sysroot: Option<&str>,
) -> String {
    let mut result = value.to_string();
    result = result.replace("{num_cpus}", &num_cpus.to_string());
    result = result.replace("{build_dir}", build_dir);
    if let Some(tools) = bootstrap_tools {
        result = result.replace("{bootstrap_tools}", tools);
    }
    if let Some(sysroot) = bootstrap_sysroot {
        result = result.replace("{bootstrap_sysroot}", sysroot);
    }
    result
}

/// Pick the repository a Git blob reference resolves against.
///
/// The manifest's own repository comes first. `fallback_path` keeps older
/// call sites and tests working when the manifest is not inside a repository.
fn environment_repository_root(manifest_path: &Path, fallback_path: &str) -> PathBuf {
    repository_root_for_path(manifest_path).unwrap_or_else(|_| PathBuf::from(fallback_path))
}

fn load_environment_blob(repo_path: &str, env_ref: &str) -> io::Result<String> {
    let output = Command::new("git")
        .args(["-C", repo_path, "cat-file", "blob", env_ref])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Failed to load environment blob {}: {}",
                env_ref,
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn load_environment_file(env_ref: &str) -> io::Result<String> {
    fs::read_to_string(env_ref).map_err(|err| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to load environment file {}: {}", env_ref, err),
        )
    })
}

#[cfg(test)]
#[path = "env_tests.rs"]
mod env_tests;
