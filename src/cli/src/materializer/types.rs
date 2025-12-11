use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::PathBuf;

use indexmap::IndexSet;

use crate::repo::detect_repo_path;

/// Specifies what to materialize - supports multiple granularity levels.
#[derive(Clone, Debug)]
pub enum MaterializeRequest {
    /// Materialize an entire bundle (capsule)
    Bundle { commit: String },
    /// Materialize a specific output category from a package
    Output { commit: String },
    /// Materialize specific files from the store
    Files { commit: String, paths: Vec<String> },
}

impl MaterializeRequest {
    /// Get the primary commit reference for this request.
    pub fn commit(&self) -> &str {
        match self {
            MaterializeRequest::Bundle { commit } => commit,
            MaterializeRequest::Output { commit } => commit,
            MaterializeRequest::Files { commit, .. } => commit,
        }
    }
}

/// Mode for materialization - determines how files are laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterializeMode {
    /// Traditional FHS-like layout: all files union-merged into target directory
    Flat,
    /// Nex structure: isolated package directories with symlink forests
    Nex,
}

/// Configuration for a materialization operation.
#[derive(Clone, Debug)]
pub struct MaterializeConfig {
    /// content store repository path
    pub repo_path: String,
    /// Target directory for materialization (logical root, used for symlink targets)
    pub target_dir: PathBuf,
    /// Physical write target (overlay upper dir). If None, use target_dir.
    /// When staging mode is active, files are written here to bypass OverlayFS device boundary.
    pub physical_root: Option<PathBuf>,
    /// How to lay out files
    pub mode: MaterializeMode,
    /// Package database path (for recording installations)
    pub db_path: Option<PathBuf>,
    /// Whether to resolve runtime dependencies transitively
    pub resolve_deps: bool,
    /// Manifest database paths for layered resolution (user -> system)
    pub manifest_db_paths: Vec<PathBuf>,
    /// Fallback stores for object/ref lookups (e.g., system repo for user installs)
    pub fallback_repo_paths: Vec<PathBuf>,
    /// Override for package directory (default: target_dir/nex/pkg)
    pub pkg_dir_override: Option<PathBuf>,
    /// Override for environment directory (default: target_dir/nex/env)
    pub env_dir_override: Option<PathBuf>,
}

impl Default for MaterializeConfig {
    fn default() -> Self {
        Self {
            repo_path: detect_repo_path(),
            target_dir: PathBuf::from("/"),
            physical_root: None,
            mode: MaterializeMode::Flat,
            db_path: None,
            resolve_deps: true,
            manifest_db_paths: Vec::new(),
            fallback_repo_paths: Vec::new(),
            pkg_dir_override: None,
            env_dir_override: None,
        }
    }
}

/// Result of resolving runtime dependencies for a set of requests.
#[derive(Clone, Debug, Default)]
pub struct RuntimeClosure {
    /// Commits that need to be materialized, in dependency order (roots first).
    /// Uses IndexSet for O(1) membership checks while preserving insertion order.
    pub commits: IndexSet<String>,
    /// Root commits (explicitly requested, not transitive deps)
    pub roots: HashSet<String>,
    /// Detailed reasons for why each commit was included (commit -> set of reasons)
    pub reasons: BTreeMap<String, BTreeSet<String>>,
    /// Unresolved requirements (library name -> set of files that need it)
    pub unresolved: BTreeMap<String, BTreeSet<String>>,
    /// For {checksum}/files commits: which specific files are needed (commit -> files)
    pub files_needed: BTreeMap<String, BTreeSet<String>>,
}

impl RuntimeClosure {
    /// Add a commit to the closure with a reason.
    pub fn add(&mut self, commit: &str, reason: String) {
        self.commits.insert(commit.to_string());
        self.reasons
            .entry(commit.to_string())
            .or_default()
            .insert(reason);
    }

    /// Add a root commit (explicitly requested, not a transitive dep).
    pub fn add_root(&mut self, commit: &str, reason: String) {
        self.roots.insert(commit.to_string());
        self.add(commit, reason);
    }

    /// Check if a commit is a root (explicitly requested).
    pub fn is_root(&self, commit: &str) -> bool {
        self.roots.contains(commit)
    }

    /// Add an unresolved requirement.
    pub fn add_unresolved(&mut self, requirement: &str, reason: String) {
        self.unresolved
            .entry(requirement.to_string())
            .or_default()
            .insert(reason);
    }

    /// Check if the closure has any unresolved dependencies.
    pub fn has_unresolved(&self) -> bool {
        !self.unresolved.is_empty()
    }

    /// Get all commits in the closure.
    pub fn all_commits(&self) -> impl Iterator<Item = &String> {
        self.commits.iter()
    }

    /// Check if empty (no commits to materialize).
    pub fn is_empty(&self) -> bool {
        self.commits.is_empty()
    }

    /// Add a file-level dependency (specific file from a commit).
    pub fn add_file_dep(&mut self, commit: &str, file: &str, reason: String) {
        self.files_needed
            .entry(commit.to_string())
            .or_default()
            .insert(file.to_string());
        self.add(commit, reason);
    }

    /// Check if a commit has file-level dependencies (vs full checkout).
    pub fn is_file_level(&self, commit: &str) -> bool {
        self.files_needed.contains_key(commit)
    }

    /// Get the specific files needed from a commit.
    pub fn get_files(&self, commit: &str) -> Option<&BTreeSet<String>> {
        self.files_needed.get(commit)
    }
}

/// Result of a materialization operation.
#[derive(Clone, Debug)]
pub struct MaterializeResult {
    /// The resolved runtime closure
    pub closure: RuntimeClosure,
    /// Files that were materialized (target path -> source commit)
    pub materialized_files: BTreeMap<PathBuf, String>,
    /// Symlinks created (for Nex mode)
    pub symlinks_created: Vec<PathBuf>,
    /// Warnings generated during materialization
    pub warnings: Vec<String>,
}

impl MaterializeResult {
    pub fn new(closure: RuntimeClosure) -> Self {
        Self {
            closure,
            materialized_files: BTreeMap::new(),
            symlinks_created: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add a warning message.
    pub fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }
}

/// Entry in the package database (manifest DB).
#[derive(Clone, Debug)]
pub struct PackageEntry {
    /// store commit reference
    pub commit: String,
    /// Short hash for display
    pub short_hash: String,
    /// Package name (if known)
    pub name: Option<String>,
    /// Installation timestamp
    pub installed_at: u64,
    /// Runtime dependencies (commit refs)
    pub runtime_deps: Vec<String>,
}
