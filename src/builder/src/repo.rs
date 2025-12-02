//! repository detection and runtime context utilities.
//!
//! implements layered search for repos and manifests:
//! 1. CWD/.nex/ - local build-time/development
//! 2. /nex/users/$USER/ - user-specific
//! 3. /nex/ - system-wide
//! 4. (future: remote repos)

use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

use zub;
use nix::unistd::Uid;

/// detect the appropriate repo path based on environment.
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

/// detect manifest directories in priority order.
/// returns all existing manifest directories for layered search.
pub fn detect_manifest_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let user = std::env::var("USER").unwrap_or_else(|_| "unknown".into());

    // 1. local build-time directory
    if Path::new("pkg").exists() {
        dirs.push(PathBuf::from("pkg"));
    }

    // 2. user manifest worktree
    let user_manifests = PathBuf::from(format!("/nex/users/{}/manifests/pkg", user));
    if user_manifests.exists() {
        dirs.push(user_manifests);
    }

    // 3. system manifest database
    if Path::new("/nex/db/pkg").exists() {
        dirs.push(PathBuf::from("/nex/db/pkg"));
    }

    dirs
}

/// detect the manifest database directory for ManifestIndex.
/// checks in order: pkg/ -> /nex/users/$USER/manifests/pkg -> /nex/db/pkg
/// returns None if no manifest directory is found.
pub fn detect_manifest_dir() -> Option<String> {
    // build-time: local pkg/ directory
    if Path::new("pkg").exists() {
        return Some("pkg".to_string());
    }

    // user manifest worktree
    let user = std::env::var("USER").unwrap_or_else(|_| "unknown".into());
    let user_manifests = format!("/nex/users/{}/manifests/pkg", user);
    if Path::new(&user_manifests).exists() {
        return Some(user_manifests);
    }

    // runtime: system manifests at /nex/db/pkg
    if Path::new("/nex/db/pkg").exists() {
        return Some("/nex/db/pkg".to_string());
    }

    None
}

/// check if running as root (to skip unshare wrapper).
pub fn is_root() -> bool {
    Uid::effective().is_root()
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
                "Repository not found: {}. Use --repo to specify the correct path.",
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
    /// primary content store repository path
    pub repo_path: PathBuf,
    /// capsule directory (where packages are checked out)
    pub pkg_path: PathBuf,
    /// environment directory (symlink forests)
    pub env_path: PathBuf,
    /// fallback repos for object/ref lookups, in priority order
    pub fallback_repos: Vec<PathBuf>,
    /// whether this is a system-wide context
    pub is_system: bool,
    /// whether staging mode is required
    pub needs_staging: bool,
    /// state directory for InstalledState
    pub var_path: PathBuf,
    /// manifest directories for layered search, in priority order
    pub manifest_dirs: Vec<PathBuf>,
    /// manifest worktree directory (for user builds, None for system)
    pub manifests_path: Option<PathBuf>,
}

impl NexContext {
    /// get the first fallback repo if any (for backward compatibility)
    pub fn fallback_repo(&self) -> Option<&PathBuf> {
        self.fallback_repos.first()
    }

    /// get the primary manifest directory if any
    pub fn primary_manifest_dir(&self) -> Option<&PathBuf> {
        self.manifest_dirs.first()
    }
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
        let mut manifest_dirs = Vec::new();
        if Path::new("pkg").exists() {
            manifest_dirs.push(PathBuf::from("pkg"));
        }

        // in build-time context, still use /nex/repo as fallback if it exists
        // (useful when building on a system that already has packages)
        let mut fallback_repos = Vec::new();
        if Path::new("/nex/repo").exists() {
            fallback_repos.push(PathBuf::from("/nex/repo"));
        }

        return Ok(NexContext {
            repo_path: PathBuf::from(".nex/repo"),
            pkg_path: PathBuf::from(".nex/pkg"),
            env_path: PathBuf::from(".nex/env"),
            fallback_repos,
            is_system: false,
            needs_staging: false,
            var_path: PathBuf::from(".nex/var"),
            manifest_dirs,
            manifests_path: None, // build-time uses local pkg/ directory
        });
    }

    if system_flag {
        if !is_root {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "--system requires root",
            ));
        }

        let mut manifest_dirs = Vec::new();
        if Path::new("/nex/db/pkg").exists() {
            manifest_dirs.push(PathBuf::from("/nex/db/pkg"));
        }

        return Ok(NexContext {
            repo_path: PathBuf::from("/nex/repo"),
            pkg_path: PathBuf::from("/nex/pkg"),
            env_path: PathBuf::from("/nex/env"),
            fallback_repos: Vec::new(), // system is the ultimate fallback
            is_system: true,
            needs_staging: true,
            var_path: PathBuf::from("/nex/var"),
            manifest_dirs,
            manifests_path: None, // system uses /nex/db/pkg
        });
    }

    // user context (including root without --system)
    let user_base = PathBuf::from(format!("/nex/users/{}", user));

    // build fallback chain: user -> system
    let mut fallback_repos = Vec::new();
    if Path::new("/nex/repo").exists() {
        fallback_repos.push(PathBuf::from("/nex/repo"));
    }

    // build manifest search chain: user manifests -> system manifests
    let mut manifest_dirs = Vec::new();
    let user_manifests_pkg = user_base.join("manifests/pkg");
    if user_manifests_pkg.exists() {
        manifest_dirs.push(user_manifests_pkg);
    }
    if Path::new("/nex/db/pkg").exists() {
        manifest_dirs.push(PathBuf::from("/nex/db/pkg"));
    }

    Ok(NexContext {
        repo_path: user_base.join("repo"),
        pkg_path: user_base.join("pkg"),
        env_path: user_base.join("env"),
        fallback_repos,
        is_system: false,
        needs_staging: false,
        var_path: user_base.join("var"),
        manifest_dirs,
        manifests_path: Some(user_base.join("manifests")),
    })
}

/// ensure user directories exist, creating them if needed.
pub fn ensure_user_dirs(ctx: &NexContext) -> io::Result<()> {
    // initialize zub repo if it doesn't exist
    let config_path = ctx.repo_path.join("config.toml");
    if !config_path.exists() {
        fs::create_dir_all(&ctx.repo_path)?;
        zub::Repo::init(&ctx.repo_path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("failed to initialize user repo: {}", e),
            )
        })?;
    }

    // create other directories
    fs::create_dir_all(&ctx.pkg_path)?;
    fs::create_dir_all(ctx.env_path.join("default/bin"))?;
    fs::create_dir_all(&ctx.var_path)?;

    // setup manifests worktree if needed
    if let Some(ref manifests_path) = ctx.manifests_path {
        setup_user_manifests_worktree(manifests_path)?;
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

/// setup user's git worktree for manifests from /nex/manifests
fn setup_user_manifests_worktree(manifests_path: &Path) -> io::Result<()> {
    if manifests_path.exists() {
        return Ok(());
    }

    let source_repo = PathBuf::from("/nex/manifests");
    if !source_repo.exists() {
        // no system manifest repo - user can manually set this up
        eprintln!(
            "Note: /nex/manifests not found. User manifests worktree not created."
        );
        return Ok(());
    }

    // create parent directory if needed
    if let Some(parent) = manifests_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // create git worktree (detached from HEAD)
    let output = Command::new("git")
        .args(["worktree", "add", "--detach"])
        .arg(manifests_path)
        .arg("HEAD")
        .current_dir(&source_repo)
        .output()?;

    if !output.status.success() {
        eprintln!(
            "Warning: Could not create manifest worktree: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        eprintln!(
            "Created manifests worktree at {}",
            manifests_path.display()
        );
    }

    Ok(())
}
