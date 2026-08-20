//! content-addressed store operations using zub.
//!
//! this module provides the interface between the nex builder and the zub
//! content-addressed filesystem store. it replaces the previous ostree-based
//! implementation.
//!
//! supports layered lookup: primary repo -> fallback chain (user -> system -> remote)

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use zub::transport::{pull_local, pull_ssh, PullOptions};
use zub::{ops::ExportOptions, Commit, Config, Hash, Repo};

fn is_cross_device_hardlink_error(err: &zub::Error) -> bool {
    match err {
        zub::Error::Io { source, .. } => source.kind() == io::ErrorKind::CrossesDevices,
        _ => false,
    }
}

fn retry_checkout_after_hardlink_failure(
    repo: &Repo,
    commit: &str,
    target: &Path,
    union: bool,
    opts: zub::ops::CheckoutOptions,
) -> io::Result<()> {
    if !union && target.exists() {
        std::fs::remove_dir_all(target)?;
    }

    zub::ops::checkout(repo, commit, target, opts).map_err(|e| io::Error::other(e.to_string()))
}

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
    source: RemoteSource,
}

#[derive(Clone, Debug)]
enum RemoteSource {
    Local(PathBuf),
    Ssh { remote: String, path: PathBuf },
}

/// zub-backed content store for nex packages.
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
                        if remotes.iter().any(|r: &ParsedRemote| r.name == remote.name) {
                            continue;
                        }
                        remotes.push(ParsedRemote {
                            name: remote.name.clone(),
                            source: remote_source(&remote.url),
                        });
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
            let opts = PullOptions {
                fetch_only: false,
                dry_run: false,
            };

            eprintln!("{}", remote.pull_message(ref_name));
            match remote.pull(&self.repo, ref_name, &opts) {
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
        self.checkout_with_mode(commit, target, union, false)
    }

    /// Checkout an immutable commit using hardlinks when the filesystem allows it.
    pub fn checkout_immutable(&self, commit: &str, target: &Path, union: bool) -> io::Result<()> {
        self.checkout_with_mode(commit, target, union, true)
    }

    fn checkout_with_mode(
        &self,
        commit: &str,
        target: &Path,
        union: bool,
        hardlink: bool,
    ) -> io::Result<()> {
        let mut opts = zub::ops::CheckoutOptions {
            force: union,
            hardlink,
            preserve_sparse: false,
        };

        // try primary repo first
        match zub::ops::checkout(&self.repo, commit, target, opts.clone()) {
            Ok(()) => return Ok(()),
            Err(zub::Error::RefNotFound(_)) => {}
            Err(e) if opts.hardlink && is_cross_device_hardlink_error(&e) => {
                opts.hardlink = false;
                retry_checkout_after_hardlink_failure(
                    &self.repo,
                    commit,
                    target,
                    union,
                    opts.clone(),
                )?;
                return Ok(());
            }
            Err(e) => return Err(io::Error::other(e.to_string())),
        }

        // try each fallback in order
        for fallback in &self.fallback_chain {
            let mut fallback_opts = opts.clone();
            match zub::ops::checkout(fallback, commit, target, fallback_opts.clone()) {
                Ok(()) => return Ok(()),
                Err(zub::Error::RefNotFound(_)) => continue,
                Err(e) if fallback_opts.hardlink && is_cross_device_hardlink_error(&e) => {
                    fallback_opts.hardlink = false;
                    retry_checkout_after_hardlink_failure(
                        fallback,
                        commit,
                        target,
                        union,
                        fallback_opts,
                    )?;
                    return Ok(());
                }
                Err(e) => return Err(io::Error::other(e.to_string())),
            }
        }

        // try to pull from remote
        if self.pull_from_remote(commit)? {
            // now it should be in primary repo
            match zub::ops::checkout(&self.repo, commit, target, opts.clone()) {
                Ok(()) => return Ok(()),
                Err(e) if opts.hardlink && is_cross_device_hardlink_error(&e) => {
                    opts.hardlink = false;
                    retry_checkout_after_hardlink_failure(&self.repo, commit, target, union, opts)?;
                    return Ok(());
                }
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

    /// Check whether an artifact ref exists in the primary or a fallback store.
    pub fn artifact_exists(&self, artifact_path: &str) -> io::Result<bool> {
        if zub::artifact_ref_exists(&self.repo, artifact_path) {
            return Ok(true);
        }
        Ok(self
            .fallback_chain
            .iter()
            .any(|repo| zub::artifact_ref_exists(repo, artifact_path)))
    }

    /// Find a commit with matching manifest metadata in the primary or a fallback store.
    pub fn find_commit_by_manifest_hash(
        &self,
        branch: &str,
        target_hash: &str,
    ) -> io::Result<Option<String>> {
        if let Some(hash) = find_commit_by_manifest_hash_in_repo(&self.repo, branch, target_hash)? {
            return Ok(Some(hash));
        }
        for repo in &self.fallback_chain {
            if let Some(hash) = find_commit_by_manifest_hash_in_repo(repo, branch, target_hash)? {
                return Ok(Some(hash));
            }
        }
        Ok(None)
    }

    /// list files in a commit (recursive directory listing).
    pub fn ls(&self, commit: &str) -> io::Result<Vec<String>> {
        // try to find which repo has this commit
        let repo = self.find_repo_with_ref(commit)?;
        let opts = zub::ops::LsTreeOptions::default();
        let entries = zub::ops::ls_tree_recursive(repo, commit, &opts)
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

impl ParsedRemote {
    fn pull(
        &self,
        dst: &Repo,
        ref_name: &str,
        opts: &PullOptions,
    ) -> Result<zub::transport::PullResult, zub::Error> {
        match &self.source {
            RemoteSource::Local(path) => {
                let src = Repo::open(path)?;
                pull_local(&src, dst, ref_name, opts)
            }
            RemoteSource::Ssh { remote, path } => pull_ssh(remote, path, dst, ref_name, opts),
        }
    }

    fn pull_message(&self, ref_name: &str) -> String {
        match &self.source {
            RemoteSource::Local(path) => format!(
                "Trying to pull {} from remote '{}' ({})...",
                ref_name,
                self.name,
                path.display()
            ),
            RemoteSource::Ssh { remote, path } => format!(
                "Trying to pull {} from remote '{}' ({}:{})...",
                ref_name,
                self.name,
                remote,
                path.display()
            ),
        }
    }
}

fn remote_source(url: &str) -> RemoteSource {
    match parse_ssh_url(url) {
        Some((remote, path)) => RemoteSource::Ssh { remote, path },
        None => RemoteSource::Local(PathBuf::from(url)),
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
/// Point a ref at the same commit another ref names.
///
/// A deployment needs a store ref so that later operations can check the
/// running system out again. `nex commit` looks for `nex/deployments/<name>`
/// (`commands/commit.rs:158`); without this, that lookup finds nothing on
/// every machine and no package can be installed persistently.
pub fn publish_ref(repo_path: &str, new_ref: &str, source_ref: &str) -> io::Result<()> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;
    let hash = zub::resolve_ref(&repo, source_ref)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;
    zub::write_ref(&repo, new_ref, &hash)
        .map_err(|e| io::Error::other(format!("failed to write ref {}: {}", new_ref, e)))
}

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
pub fn checkout_into(repo_path: &str, commit: &str, dest: &Path, union: bool) -> io::Result<()> {
    checkout_into_with_fallbacks(repo_path, &[], commit, dest, union, false)
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
    verbose: bool,
) -> io::Result<()> {
    use crate::refs::PackageRef;

    let fallback_paths: Vec<PathBuf> = fallback_repos.iter().map(PathBuf::from).collect();

    // check if this ref has an internal path that needs partial extraction
    if let Ok(pkg_ref) = PackageRef::parse(commit) {
        if let Some(subpath) = pkg_ref.internal_path() {
            let base_ref = pkg_ref.commit_ref();
            if verbose {
                println!(
                    "Extracting {} from {} into {}",
                    subpath,
                    base_ref,
                    dest.display()
                );
            }

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
                false,
            );
        }
    }

    // full checkout for refs without internal paths
    if verbose {
        println!(
            "Checking out {} into {} (union: {})",
            commit,
            dest.display(),
            union
        );
    }
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
    let mut opts = zub::ops::ExportOptions {
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
        Err(e) if opts.hardlink && is_cross_device_hardlink_error(&e) => {
            opts.hardlink = false;
            zub::ops::export_path(&repo, commit, src_path, dest, opts)
                .map_err(|e| io::Error::other(e.to_string()))?;
            return Ok(());
        }
        Err(e) => return Err(io::Error::other(e.to_string())),
    }

    // try fallbacks
    for fallback_path in fallback_paths {
        if let Ok(fallback) = Repo::open(fallback_path) {
            let mut fb_opts = opts.clone();
            match zub::ops::export_path(&fallback, commit, src_path, dest, fb_opts.clone()) {
                Ok(()) => return Ok(()),
                Err(zub::Error::RefNotFound(_)) => continue,
                Err(e) if fb_opts.hardlink && is_cross_device_hardlink_error(&e) => {
                    fb_opts.hardlink = false;
                    zub::ops::export_path(&fallback, commit, src_path, dest, fb_opts)
                        .map_err(|e| io::Error::other(e.to_string()))?;
                    return Ok(());
                }
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
/// returns the tree hash for artifact creation
pub fn commit_tree(
    repo_path: &str,
    branch: &str,
    tree_path: &Path,
    metadata: &[(String, String)],
) -> io::Result<Hash> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    // convert metadata to &[(&str, &str)]
    let metadata_refs: Vec<(&str, &str)> = metadata
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let commit_hash =
        zub::ops::commit_with_metadata(&repo, tree_path, branch, Some(""), None, &metadata_refs)
            .map_err(|e| io::Error::other(e.to_string()))?;

    // get tree hash from the commit
    let commit =
        zub::read_commit(&repo, &commit_hash).map_err(|e| io::Error::other(e.to_string()))?;

    Ok(commit.tree)
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

/// create an artifact linking a manifest to its build output tree
///
/// artifact_path is the full hierarchical path for the artifact ref,
/// e.g. "x86_64/pkg/libs/foo/1.0/<manifest_hash>/outputs/bin"
pub fn create_artifact(
    repo_path: &str,
    tree_hash: &Hash,
    manifest_hash: &str,
    output: &str,
    artifact_path: &str,
) -> io::Result<Hash> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    let manifest_hash_obj = Hash::from_hex(manifest_hash)
        .map_err(|e| io::Error::other(format!("invalid manifest hash: {}", e)))?;

    let artifact = zub::Artifact::new(*tree_hash, manifest_hash_obj, output);
    let artifact_hash =
        zub::write_artifact(&repo, &artifact).map_err(|e| io::Error::other(e.to_string()))?;

    // write the artifact ref for O(1) lookup
    zub::write_artifact_ref(&repo, artifact_path, &artifact_hash)
        .map_err(|e| io::Error::other(e.to_string()))?;

    println!("Created artifact: {} -> {}", artifact_path, artifact_hash);
    Ok(artifact_hash)
}

/// get tree hash from an existing branch
pub fn get_branch_tree(repo_path: &str, branch: &str) -> io::Result<Hash> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    let commit_hash =
        zub::resolve_ref(&repo, branch).map_err(|e| io::Error::other(e.to_string()))?;

    let commit =
        zub::read_commit(&repo, &commit_hash).map_err(|e| io::Error::other(e.to_string()))?;

    Ok(commit.tree)
}

/// lookup an artifact by path, returning the tree hash if found
pub fn lookup_artifact(repo_path: &str, artifact_path: &str) -> io::Result<Option<Hash>> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    if !zub::artifact_ref_exists(&repo, artifact_path) {
        return Ok(None);
    }

    let artifact_hash = zub::read_artifact_ref(&repo, artifact_path)
        .map_err(|e| io::Error::other(e.to_string()))?;

    let artifact =
        zub::read_artifact(&repo, &artifact_hash).map_err(|e| io::Error::other(e.to_string()))?;

    Ok(Some(artifact.tree))
}

/// checkout an artifact directly to a target directory
pub fn checkout_artifact(
    repo_path: &str,
    artifact_path: &str,
    target: &Path,
    force: bool,
) -> io::Result<bool> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    let tree_hash = match lookup_artifact(repo_path, artifact_path)? {
        Some(h) => h,
        None => return Ok(false),
    };

    let opts = zub::ops::CheckoutOptions {
        force,
        hardlink: false,
        preserve_sparse: false,
    };

    zub::ops::checkout_from_tree_hash(&repo, &tree_hash, target, opts)
        .map_err(|e| io::Error::other(e.to_string()))?;

    Ok(true)
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
/// checks HEAD commit first, then walks history if available.
pub fn find_commit_by_manifest_hash(
    repo_path: &str,
    branch: &str,
    target_hash: &str,
) -> io::Result<Option<String>> {
    let repo = Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

    find_commit_by_manifest_hash_in_repo(&repo, branch, target_hash)
}

fn find_commit_by_manifest_hash_in_repo(
    repo: &Repo,
    branch: &str,
    target_hash: &str,
) -> io::Result<Option<String>> {
    // resolve branch to commit hash
    let head_hash = match zub::resolve_ref(&repo, branch) {
        Ok(h) => h,
        Err(_) => return Ok(None), // branch doesn't exist
    };

    // check HEAD commit directly (most common case, avoids history walk)
    if let Ok(commit) = zub::read_commit(&repo, &head_hash) {
        if let Some(hash) = commit.metadata.get("nex.manifest.hash") {
            if hash == target_hash {
                return Ok(Some(head_hash.to_hex()));
            }
        }
    }

    // walk history only if HEAD didn't match (handles rebuilds with different manifest)
    let log = match zub::ops::log(&repo, branch, None) {
        Ok(entries) => entries,
        Err(_) => return Ok(None), // history incomplete, HEAD already checked
    };

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

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
