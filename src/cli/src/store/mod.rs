//! content-addressed store operations using bog.
//!
//! this module provides the interface between the nex builder and the bog
//! content-addressed filesystem store. it replaces the previous ostree-based
//! implementation.
//!
//! supports layered lookup: primary repo -> fallback chain (user -> system -> remote)

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use zub::transport::{pull_ssh, PullOptions};
use zub::{ops::ExportOptions, Commit, Config, Hash, Repo};

/// parse an SSH URL into (remote, path) components.
/// supports formats:
///   - ssh://user@host/path -> ("user@host", "/path")
///   - ssh://host/path -> ("host", "/path")
///   - user@host:/path -> ("user@host", "/path")
fn parse_ssh_url(url: &str) -> Option<(String, PathBuf)> {
    if let Some(rest) = url.strip_prefix("ssh://") {
        // ssh://[user@]host/path
        let slash_pos = rest.find('/')?;
        let remote = rest[..slash_pos].to_string();
        let path = PathBuf::from(&rest[slash_pos..]);
        Some((remote, path))
    } else if url.contains(":/") && url.contains('@') {
        // user@host:/path (scp-style)
        let colon_pos = url.find(":/")?;
        let remote = url[..colon_pos].to_string();
        let path = PathBuf::from(&url[colon_pos + 1..]);
        Some((remote, path))
    } else {
        None
    }
}

/// a configured remote with parsed URL components
#[derive(Clone, Debug)]
pub struct ParsedRemote {
    pub name: String,
    pub remote: String, // user@host or host
    pub path: PathBuf,  // repo path on remote
}

/// bog-backed content store for nex packages.
/// supports multiple fallback repos for layered search and remote fetching.
pub struct Store {
    pub path: PathBuf,
    repo: Repo,
    fallback_chain: Vec<Repo>,
    remotes: Vec<ParsedRemote>,
}

impl Store {
    /// open an existing store.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::open_with_fallback_chain(path, &[])
    }

    /// open a store with a single fallback repo (backward compatibility).
    pub fn open_with_fallback(path: impl AsRef<Path>, fallback: Option<&Path>) -> io::Result<Self> {
        let fallbacks: Vec<PathBuf> = fallback.map(|p| vec![p.to_path_buf()]).unwrap_or_default();
        Self::open_with_fallback_chain(path, &fallbacks)
    }

    /// open a store with multiple fallback repos for layered lookup.
    pub fn open_with_fallback_chain(
        path: impl AsRef<Path>,
        fallback_paths: &[PathBuf],
    ) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();

        let repo = Repo::open(&path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("not a zub repository: {} ({})", path.display(), e),
            )
        })?;

        let mut fallback_chain = Vec::new();
        for fb_path in fallback_paths {
            if fb_path.exists() {
                match Repo::open(fb_path) {
                    Ok(fb_repo) => fallback_chain.push(fb_repo),
                    Err(e) => {
                        eprintln!(
                            "Warning: could not open fallback repo {}: {}",
                            fb_path.display(),
                            e
                        );
                    }
                }
            }
        }

        // load remotes from config (check primary repo and all fallbacks)
        let mut remotes = Vec::new();
        let mut config_paths = vec![path.join("config.toml")];
        for fb_path in fallback_paths {
            config_paths.push(fb_path.join("config.toml"));
        }
        for config_path in config_paths {
            if config_path.exists() {
                if let Ok(config) = Config::load(&config_path) {
                    for remote in &config.remotes {
                        if let Some((host, rpath)) = parse_ssh_url(&remote.url) {
                            // avoid duplicates
                            if !remotes.iter().any(|r: &ParsedRemote| r.name == remote.name) {
                                remotes.push(ParsedRemote {
                                    name: remote.name.clone(),
                                    remote: host,
                                    path: rpath,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            path,
            repo,
            fallback_chain,
            remotes,
        })
    }

    /// initialize a new store at the given path.
    pub fn init(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let repo = Repo::init(&path)
            .map_err(|e| io::Error::other(format!("failed to initialize zub repository: {}", e)))?;
        Ok(Self {
            path,
            repo,
            fallback_chain: Vec::new(),
            remotes: Vec::new(),
        })
    }

    /// try to pull a ref from configured remotes.
    /// returns Ok(true) if pulled successfully, Ok(false) if not found on any remote.
    pub fn pull_from_remote(&self, ref_name: &str) -> io::Result<bool> {
        if self.remotes.is_empty() {
            return Ok(false);
        }

        for remote in &self.remotes {
            eprintln!(
                "Trying to pull {} from remote '{}' ({}:{})...",
                ref_name,
                remote.name,
                remote.remote,
                remote.path.display()
            );

            let opts = PullOptions {
                fetch_only: false,
                dry_run: false,
            };

            match pull_ssh(&remote.remote, &remote.path, &self.repo, ref_name, &opts) {
                Ok(result) => {
                    eprintln!(
                        "Pulled {} from '{}': {} bytes, {} objects",
                        ref_name, remote.name, result.stats.bytes_transferred, result.stats.copied
                    );
                    return Ok(true);
                }
                Err(e) => {
                    eprintln!("Could not pull from '{}': {}", remote.name, e);
                    // try next remote
                }
            }
        }

        Ok(false)
    }

    /// list all refs, optionally filtered by pattern.
    /// merges refs from primary and all fallback repos.
    pub fn refs(&self, pattern: Option<&str>) -> io::Result<Vec<String>> {
        let mut all_refs = match pattern {
            Some(p) => zub::list_refs_matching(&self.repo, p),
            None => zub::list_refs(&self.repo),
        }
        .map_err(|e| io::Error::other(e.to_string()))?;

        // include refs from all fallback repos
        for fallback in &self.fallback_chain {
            let fallback_refs = match pattern {
                Some(p) => zub::list_refs_matching(fallback, p),
                None => zub::list_refs(fallback),
            }
            .map_err(|e| io::Error::other(e.to_string()))?;

            for r in fallback_refs {
                if !all_refs.contains(&r) {
                    all_refs.push(r);
                }
            }
        }

        Ok(all_refs)
    }

    /// resolve a ref to its commit hash.
    /// tries primary repo first, then each fallback in order, then remotes.
    pub fn resolve_ref(&self, ref_name: &str) -> io::Result<String> {
        // try primary repo
        if let Ok(hash) = zub::resolve_ref(&self.repo, ref_name) {
            return Ok(hash.to_hex());
        }

        // try each fallback in order
        for fallback in &self.fallback_chain {
            if let Ok(hash) = zub::resolve_ref(fallback, ref_name) {
                return Ok(hash.to_hex());
            }
        }

        // try to pull from remote
        if self.pull_from_remote(ref_name)? {
            // now it should be in primary repo
            if let Ok(hash) = zub::resolve_ref(&self.repo, ref_name) {
                return Ok(hash.to_hex());
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("ref not found: {}", ref_name),
        ))
    }

    /// checkout a commit to target directory.
    /// tries primary repo first, then each fallback in order, then remotes.
    pub fn checkout(&self, commit: &str, target: &Path, union: bool) -> io::Result<()> {
        let opts = zub::ops::CheckoutOptions {
            force: union,
            hardlink: true,
            preserve_sparse: false,
        };

        // try primary repo first
        match zub::ops::checkout(&self.repo, commit, target, opts.clone()) {
            Ok(()) => return Ok(()),
            Err(zub::Error::RefNotFound(_)) => {}
            Err(e) => return Err(io::Error::other(e.to_string())),
        }

        // try each fallback in order
        for fallback in &self.fallback_chain {
            match zub::ops::checkout(fallback, commit, target, opts.clone()) {
                Ok(()) => return Ok(()),
                Err(zub::Error::RefNotFound(_)) => continue,
                Err(e) => return Err(io::Error::other(e.to_string())),
            }
        }

        // try to pull from remote
        if self.pull_from_remote(commit)? {
            // now it should be in primary repo
            match zub::ops::checkout(&self.repo, commit, target, opts) {
                Ok(()) => return Ok(()),
                Err(e) => return Err(io::Error::other(e.to_string())),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("commit not found in any repo: {}", commit),
        ))
    }

    /// get commit metadata.
    pub fn get_commit_info(&self, commit: &str) -> io::Result<CommitInfo> {
        let (hash, repo) = self.resolve_ref_to_hash_and_repo(commit)?;
        let commit_obj =
            zub::read_commit(repo, &hash).map_err(|e| io::Error::other(e.to_string()))?;

        Ok(CommitInfo {
            tree: commit_obj.tree.to_hex(),
            parent: commit_obj.parents.first().map(|h| h.to_hex()),
            author: commit_obj.author.clone(),
            timestamp: commit_obj.timestamp,
            message: commit_obj.message.clone(),
            metadata: commit_obj.metadata.clone(),
        })
    }

    /// get a specific metadata key from a commit.
    pub fn get_metadata(&self, commit: &str, key: &str) -> io::Result<Option<String>> {
        let info = self.get_commit_info(commit)?;
        Ok(info.metadata.get(key).cloned())
    }

    /// list files in a commit (recursive directory listing).
    pub fn ls(&self, commit: &str) -> io::Result<Vec<String>> {
        // try to find which repo has this commit
        let repo = self.find_repo_with_ref(commit)?;
        let entries = zub::ops::ls_tree_recursive(repo, commit)
            .map_err(|e| io::Error::other(e.to_string()))?;

        Ok(entries.into_iter().map(|e| e.path).collect())
    }

    /// read a file from a commit.
    pub fn cat(&self, commit: &str, path: &str) -> io::Result<Vec<u8>> {
        let (hash, repo) = self.resolve_ref_to_hash_and_repo(commit)?;
        let commit_obj =
            zub::read_commit(repo, &hash).map_err(|e| io::Error::other(e.to_string()))?;

        // navigate tree to find the file
        self.cat_from_tree(repo, &commit_obj.tree, path)
    }

    /// check if a commit/ref exists.
    pub fn exists(&self, commit_or_ref: &str) -> bool {
        self.resolve_ref(commit_or_ref).is_ok()
    }

    /// get the underlying repo reference
    pub fn repo(&self) -> &Repo {
        &self.repo
    }

    // helper: find which repo contains this ref
    fn find_repo_with_ref(&self, ref_name: &str) -> io::Result<&Repo> {
        if zub::ref_exists(&self.repo, ref_name) {
            return Ok(&self.repo);
        }

        for fallback in &self.fallback_chain {
            if zub::ref_exists(fallback, ref_name) {
                return Ok(fallback);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("ref not found: {}", ref_name),
        ))
    }

    // helper: resolve ref name or hash string to Hash, and return which repo has it
    fn resolve_ref_to_hash_and_repo(&self, ref_or_hash: &str) -> io::Result<(Hash, &Repo)> {
        // try primary repo
        if let Ok(hash) = zub::resolve_ref(&self.repo, ref_or_hash) {
            return Ok((hash, &self.repo));
        }

        // try each fallback in order
        for fallback in &self.fallback_chain {
            if let Ok(hash) = zub::resolve_ref(fallback, ref_or_hash) {
                return Ok((hash, fallback));
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("ref not found: {}", ref_or_hash),
        ))
    }

    // helper: find and read a file from a tree
    fn cat_from_tree(&self, repo: &Repo, tree_hash: &Hash, path: &str) -> io::Result<Vec<u8>> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();

        if parts.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty path"));
        }

        let tree = zub::read_tree(repo, tree_hash).map_err(|e| io::Error::other(e.to_string()))?;

        if parts.len() == 1 {
            // looking for a file in this directory
            let filename = parts[0];
            for entry in tree.entries() {
                if entry.name == filename {
                    if let Some(hash) = entry.kind.hash() {
                        return zub::read_blob(repo, hash)
                            .map_err(|e| io::Error::other(e.to_string()));
                    }
                }
            }
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("file not found: {}", path),
            ));
        }

        // looking in a subdirectory
        let dirname = parts[0];
        let rest = parts[1..].join("/");

        for entry in tree.entries() {
            if entry.name == dirname {
                if let zub::EntryKind::Directory { hash, .. } = &entry.kind {
                    return self.cat_from_tree(repo, hash, &rest);
                }
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("directory not found: {}", dirname),
        ))
    }
}

/// commit metadata info
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub tree: String,
    pub parent: Option<String>,
    pub author: String,
    pub timestamp: i64,
    pub message: String,
    pub metadata: BTreeMap<String, String>,
}

// ============================================================================
// high-level operations (replaces ostree/mod.rs functions)
// ============================================================================

/// check if a branch/ref exists in the repository
pub fn ensure_branch_exists(repo_path: &str, branch: &str) -> io::Result<()> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    if zub::ref_exists(&repo, branch) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("branch {} missing in {}", branch, repo_path),
        ))
    }
}

/// get the commit ID (hash) for a branch
pub fn get_commit_id(repo_path: &str, branch: &str) -> io::Result<String> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    let hash = zub::resolve_ref(&repo, branch)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    Ok(hash.to_hex())
}

/// get a metadata value from a commit
pub fn get_commit_metadata(repo_path: &str, commit: &str, key: &str) -> io::Result<String> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    let hash = zub::resolve_ref(&repo, commit)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    let commit_obj = zub::read_commit(&repo, &hash).map_err(|e| io::Error::other(e.to_string()))?;

    commit_obj.metadata.get(key).cloned().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("metadata key {} not found for {}", key, commit),
        )
    })
}

/// checkout a commit into a directory
pub fn checkout_into(
    repo_path: &str,
    commit: &str,
    dest: &Path,
    union: bool,
    _allow_noent: bool,
) -> io::Result<()> {
    checkout_into_with_fallbacks(repo_path, &[], commit, dest, union)
}

/// checkout a commit into a directory, with fallback repos for dependency lookups.
/// handles refs with internal paths (e.g., `outputs/bin/usr/bin/foo`) by extracting
/// only the specified subpath from the commit.
pub fn checkout_into_with_fallbacks(
    repo_path: &str,
    fallback_repos: &[String],
    commit: &str,
    dest: &Path,
    union: bool,
) -> io::Result<()> {
    use crate::refs::PackageRef;

    let fallback_paths: Vec<PathBuf> = fallback_repos.iter().map(PathBuf::from).collect();

    // check if this ref has an internal path that needs partial extraction
    if let Ok(pkg_ref) = PackageRef::parse(commit) {
        if let Some(subpath) = pkg_ref.internal_path() {
            let base_ref = pkg_ref.commit_ref();
            println!(
                "Extracting {} from {} into {}",
                subpath,
                base_ref,
                dest.display()
            );

            let target_path = dest.join(subpath);
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            return export_path_with_fallbacks(
                repo_path,
                &fallback_paths,
                &base_ref,
                subpath,
                &target_path,
                true, // hardlink
            );
        }
    }

    // full checkout for refs without internal paths
    println!(
        "Checking out {} into {} (union: {})",
        commit,
        dest.display(),
        union
    );
    let store = Store::open_with_fallback_chain(repo_path, &fallback_paths)?;
    store.checkout(commit, dest, union)
}

/// export a single path from a commit, trying fallback repos if needed.
fn export_path_with_fallbacks(
    repo_path: &str,
    fallback_paths: &[PathBuf],
    commit: &str,
    src_path: &str,
    dest: &Path,
    hardlink: bool,
) -> io::Result<()> {
    let opts = zub::ops::ExportOptions {
        overwrite: true,
        hardlink,
        preserve_sparse: false,
    };

    // try primary repo
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    match zub::ops::export_path(&repo, commit, src_path, dest, opts.clone()) {
        Ok(()) => return Ok(()),
        Err(zub::Error::RefNotFound(_)) => {}
        Err(e) => return Err(io::Error::other(e.to_string())),
    }

    // try fallbacks
    for fallback_path in fallback_paths {
        if let Ok(fallback) = Repo::open(fallback_path) {
            match zub::ops::export_path(&fallback, commit, src_path, dest, opts.clone()) {
                Ok(()) => return Ok(()),
                Err(zub::Error::RefNotFound(_)) => continue,
                Err(e) => return Err(io::Error::other(e.to_string())),
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("commit {} not found in any repo", commit),
    ))
}

/// export a single path from a commit/ref to the filesystem
pub fn export_path(
    repo_path: &str,
    commit: &str,
    src_path: &str,
    dest: &Path,
    hardlink: bool,
) -> io::Result<()> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    zub::ops::export_path(
        &repo,
        commit,
        src_path,
        dest,
        ExportOptions {
            overwrite: true,
            hardlink,
            preserve_sparse: false,
        },
    )
    .map_err(|e| io::Error::other(e.to_string()))
}

/// commit a directory to the store
pub fn commit_tree(
    repo_path: &str,
    branch: &str,
    tree_path: &Path,
    metadata: &[(String, String)],
) -> io::Result<()> {
    println!("Committing {} to branch {}", tree_path.display(), branch);

    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    // convert metadata to &[(&str, &str)]
    let metadata_refs: Vec<(&str, &str)> = metadata
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    zub::ops::commit_with_metadata(&repo, tree_path, branch, Some(""), None, &metadata_refs)
        .map_err(|e| io::Error::other(e.to_string()))?;

    Ok(())
}

/// rewrite branch metadata without changing the tree
pub fn rewrite_branch_metadata(
    repo_path: &str,
    branch: &str,
    metadata: &[(String, String)],
) -> io::Result<()> {
    println!("Rewriting metadata for {}", branch);

    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    // read current commit
    let hash = zub::resolve_ref(&repo, branch)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    let old_commit = zub::read_commit(&repo, &hash).map_err(|e| io::Error::other(e.to_string()))?;

    // create new commit with same tree but updated metadata
    let mut new_commit = Commit::new(
        old_commit.tree,
        vec![hash], // parent is the old commit
        &old_commit.author,
        &old_commit.message,
    );

    for (key, value) in metadata {
        new_commit = new_commit.with_metadata(key, value);
    }

    let new_hash =
        zub::write_commit(&repo, &new_commit).map_err(|e| io::Error::other(e.to_string()))?;

    zub::write_ref(&repo, branch, &new_hash).map_err(|e| io::Error::other(e.to_string()))?;

    Ok(())
}

/// get metadata value from a branch
pub fn get_branch_metadata(repo_path: &str, branch: &str, key: &str) -> io::Result<String> {
    get_commit_metadata(repo_path, branch, key)
}

/// encode a list of strings as JSON for metadata
pub fn encode_metadata_list(values: &[String]) -> io::Result<Option<String>> {
    if values.is_empty() {
        return Ok(None);
    }
    let json = serde_json::to_string(values)
        .map_err(|e| io::Error::other(format!("failed to encode metadata: {}", e)))?;
    Ok(Some(json))
}

/// read build checksum from a commit
pub fn read_checksum_from_commit(repo_path: &str, commit: &str) -> io::Result<String> {
    get_commit_metadata(repo_path, commit, "nex.build.checksum")
}

/// find a commit in history by its manifest hash.
pub fn find_commit_by_manifest_hash(
    repo_path: &str,
    branch: &str,
    target_hash: &str,
) -> io::Result<Option<String>> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    // get commit log for the branch
    let log = match zub::ops::log(&repo, branch, None) {
        Ok(entries) => entries,
        Err(_) => return Ok(None), // branch doesn't exist
    };

    // check each commit's manifest hash
    for entry in log {
        if let Some(hash) = entry.commit.metadata.get("nex.manifest.hash") {
            if hash == target_hash {
                return Ok(Some(entry.hash.to_hex()));
            }
        }
    }

    Ok(None)
}

/// initialize a new repository if it doesn't exist
pub fn init_repo_if_needed(repo_path: &str) -> io::Result<()> {
    let path = Path::new(repo_path);
    if path.join("config.toml").exists() {
        return Ok(()); // already initialized
    }

    std::fs::create_dir_all(path)?;
    Repo::init(path).map_err(|e| io::Error::other(e.to_string()))?;

    Ok(())
}
