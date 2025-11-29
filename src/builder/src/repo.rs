//! repository detection and runtime context utilities.

use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
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

/// execution context for nex commands (user vs system).
#[derive(Debug, Clone)]
pub struct NexContext {
    /// OSTree repository path
    pub repo_path: PathBuf,
    /// capsule directory (where packages are checked out)
    pub pkg_path: PathBuf,
    /// environment directory (symlink forests)
    pub env_path: PathBuf,
    /// fallback repo for object/ref lookups (system repo for user context)
    pub fallback_repo: Option<PathBuf>,
    /// whether this is a system-wide context
    pub is_system: bool,
    /// whether staging mode is required
    pub needs_staging: bool,
}

/// detect execution context based on user and flags.
/// - regular user: operates on /nex/users/$USER/, no staging required
/// - root without --system: operates on /nex/users/root/, no staging required
/// - root with --system: operates on /nex/, staging required
pub fn detect_context(system_flag: bool) -> io::Result<NexContext> {
    let is_root = Uid::effective().is_root();
    let user = std::env::var("USER").unwrap_or_else(|_| "unknown".into());

    // build-time context: if .nex/repo exists, use it (for development)
    if Path::new(".nex/repo").exists() && !system_flag {
        return Ok(NexContext {
            repo_path: PathBuf::from(".nex/repo"),
            pkg_path: PathBuf::from(".nex/pkg"),
            env_path: PathBuf::from(".nex/env"),
            fallback_repo: None,
            is_system: false,
            needs_staging: false,
        });
    }

    if system_flag {
        if !is_root {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "--system requires root",
            ));
        }
        return Ok(NexContext {
            repo_path: PathBuf::from("/nex/repo"),
            pkg_path: PathBuf::from("/nex/pkg"),
            env_path: PathBuf::from("/nex/env"),
            fallback_repo: None,
            is_system: true,
            needs_staging: true,
        });
    }

    // user context (including root without --system)
    let user_base = PathBuf::from(format!("/nex/users/{}", user));
    Ok(NexContext {
        repo_path: user_base.join("repo"),
        pkg_path: user_base.join("pkg"),
        env_path: user_base.join("env"),
        fallback_repo: Some(PathBuf::from("/nex/repo")),
        is_system: false,
        needs_staging: false,
    })
}

/// ensure user directories exist, creating them if needed.
pub fn ensure_user_dirs(ctx: &NexContext) -> io::Result<()> {
    // create directory structure
    fs::create_dir_all(ctx.repo_path.join("objects"))?;
    fs::create_dir_all(ctx.repo_path.join("refs/heads"))?;
    fs::create_dir_all(&ctx.pkg_path)?;
    fs::create_dir_all(ctx.env_path.join("default/bin"))?;

    // write minimal repo config if not exists
    let config_path = ctx.repo_path.join("config");
    if !config_path.exists() {
        fs::write(&config_path, "[core]\nrepo_version=1\nmode=bare-user\n")?;
    }

    // create ~/.nex convenience symlink
    if let Ok(home) = std::env::var("HOME") {
        let symlink_path = PathBuf::from(&home).join(".nex");
        if !symlink_path.exists() {
            if let Some(parent) = ctx.repo_path.parent() {
                let _ = symlink(parent, &symlink_path);
            }
        }
    }

    Ok(())
}
