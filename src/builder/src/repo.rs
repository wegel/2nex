//! repository detection and runtime context utilities.

use std::io;
use std::path::Path;
use std::process::Command;

use nix::unistd::Uid;

/// detect the appropriate OSTree repo path based on environment.
/// checks in order: .nex/repo (local build-time) -> /nex/repo (runtime)
pub fn detect_repo_path() -> String {
    // build-time: local .nex/repo takes priority
    if Path::new(".nex/repo").exists() {
        return ".nex/repo".to_string();
    }
    // runtime: system has repo at /nex/repo
    if Path::new("/nex/repo").exists() {
        return "/nex/repo".to_string();
    }
    // default for new builds
    ".nex/repo".to_string()
}

/// detect the manifest database directory for ManifestIndex.
/// checks in order: /nex/db/pkg -> pkg/
/// returns None if no manifest directory is found.
pub fn detect_manifest_dir() -> Option<String> {
    // runtime: VM has manifests deployed at /nex/db/pkg
    if Path::new("/nex/db/pkg").exists() {
        return Some("/nex/db/pkg".to_string());
    }
    // build-time: local pkg/ directory
    if Path::new("pkg").exists() {
        return Some("pkg".to_string());
    }
    None
}

/// check if running as root (to skip unshare wrapper).
pub fn is_root() -> bool {
    Uid::effective().is_root()
}

/// create an ostree command, wrapping with unshare when not root.
pub fn ostree_command() -> Command {
    if is_root() {
        Command::new("ostree")
    } else {
        let mut cmd = Command::new("unshare");
        cmd.args(["--user", "--map-root-user", "--", "ostree"]);
        cmd
    }
}

/// resolve repo path from optional argument, using auto-detection as fallback.
/// validates that the repo exists and returns an absolute path.
pub fn resolve_repo_path(repo_arg: Option<&str>) -> io::Result<String> {
    let repo_path = match repo_arg {
        Some(path) => path.to_string(),
        None => detect_repo_path(),
    };

    let path = Path::new(&repo_path);
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "OSTree repository not found: {}. Use --repo to specify the correct path.",
                repo_path
            ),
        ));
    }

    // canonicalize to absolute path for use in unshare/bubblewrap contexts
    let abs_path = path.canonicalize().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to canonicalize repo path {}: {}", repo_path, e),
        )
    })?;

    Ok(abs_path.to_string_lossy().to_string())
}
