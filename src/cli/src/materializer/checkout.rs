use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use tempfile::TempDir;

use crate::manifest::ManifestIndex;
use crate::store::Store;

use super::flatten::flatten_capsule_precomputed;
use super::types::{MaterializeConfig, MaterializeMode, MaterializeResult, RuntimeClosure};

/// Package identity for grouping outputs during materialization.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct PackageId {
    /// Package path (e.g., "core/init/systemd-with-dbus/257.5")
    path: String,
    /// Manifest hash (shared by all outputs of the same build)
    manifest_hash: String,
}

/// Calculate relative path from `base` to `target`.
fn diff_paths(target: &Path, base: &Path) -> Option<PathBuf> {
    let target = target.canonicalize().ok()?;
    let base = base.canonicalize().ok()?;

    let mut target_components = target.components().peekable();
    let mut base_components = base.components().peekable();

    // skip common prefix
    while target_components.peek() == base_components.peek() {
        target_components.next();
        if base_components.next().is_none() {
            break;
        }
    }

    // count remaining base components (need to go up)
    let ups = base_components
        .filter(|c| matches!(c, Component::Normal(_)))
        .count();

    let mut result = PathBuf::new();
    for _ in 0..ups {
        result.push("..");
    }
    for component in target_components {
        result.push(component);
    }

    Some(result)
}

/// Checkout commits from the closure to the target directory.
pub fn checkout_closure(
    config: &MaterializeConfig,
    closure: &RuntimeClosure,
) -> io::Result<MaterializeResult> {
    let mut result = MaterializeResult::new(closure.clone());

    // load manifest index for flattening (only needed for Nex mode)
    let manifest_index = if config.mode == MaterializeMode::Nex {
        if !config.manifest_db_paths.is_empty() {
            Some(ManifestIndex::load_layered(&config.manifest_db_paths)?)
        } else {
            None
        }
    } else {
        None
    };

    match config.mode {
        MaterializeMode::Flat => checkout_flat(config, closure, &mut result)?,
        MaterializeMode::Nex => {
            checkout_nex(config, closure, &mut result, manifest_index.as_ref())?
        }
    }

    Ok(result)
}

/// Flat checkout: union merge all commits into target directory.
fn checkout_flat(
    config: &MaterializeConfig,
    closure: &RuntimeClosure,
    _result: &mut MaterializeResult,
) -> io::Result<()> {
    // ensure target directory exists
    fs::create_dir_all(&config.target_dir)?;

    // checkout each commit, union merging into target
    for commit in closure.all_commits() {
        if let Some(files) = closure.get_files(commit) {
            // file-level checkout: only extract specific files
            let files_vec: Vec<String> = files.iter().cloned().collect();
            println!("  Checking out {} file(s) from {}", files_vec.len(), commit);
            checkout_files(
                &config.repo_path,
                commit,
                &files_vec,
                &config.target_dir,
                &config.fallback_repo_paths,
            )?;
        } else {
            // full checkout for root commits
            println!("  Checking out {} (flat)", commit);
            checkout_commit_flat(
                &config.repo_path,
                commit,
                &config.target_dir,
                &config.fallback_repo_paths,
            )?;
        }
    }

    Ok(())
}

/// Checkout a single commit in flat mode (union merge).
fn checkout_commit_flat(
    repo_path: &str,
    commit: &str,
    target_dir: &Path,
    fallback_repos: &[PathBuf],
) -> io::Result<()> {
    let store = Store::open_with_fallback_chain(repo_path, fallback_repos)?;
    store.checkout(commit, target_dir, true)
}

/// Nex checkout: isolated package directories with symlink forests.
/// Groups outputs by manifest hash so relative symlinks within a package work.
///
/// When `physical_root` is set (staging mode), files are written there and overlays
/// make them visible at the logical paths.
fn checkout_nex(
    config: &MaterializeConfig,
    closure: &RuntimeClosure,
    result: &mut MaterializeResult,
    manifest_index: Option<&ManifestIndex>,
) -> io::Result<()> {
    // physical root is where we actually write (overlay upper dir or target_dir)
    let physical_root = config.physical_root.as_ref().unwrap_or(&config.target_dir);
    // logical root is used for symlink target calculation (always target_dir)
    let logical_root = &config.target_dir;

    // physical paths for writing - use overrides if provided
    let physical_nex_pkg = config
        .pkg_dir_override
        .clone()
        .unwrap_or_else(|| physical_root.join("nex/pkg"));
    let physical_nex_env = config
        .env_dir_override
        .clone()
        .unwrap_or_else(|| physical_root.join("nex/env"));
    fs::create_dir_all(&physical_nex_pkg)?;
    fs::create_dir_all(&physical_nex_env)?;

    // logical paths (for checking existing packages via overlay view)
    let logical_nex_pkg = config
        .pkg_dir_override
        .clone()
        .unwrap_or_else(|| logical_root.join("nex/pkg"));

    // group commits by package identity (path + manifest hash)
    // this ensures bin/lib outputs from the same build go to the same directory
    let mut packages: BTreeMap<PackageId, Vec<String>> = BTreeMap::new();
    // track which packages are roots (explicitly requested, not just deps)
    let mut root_packages: std::collections::HashSet<PackageId> = std::collections::HashSet::new();

    for commit in closure.all_commits() {
        let pkg_id = get_package_id(&config.repo_path, commit, &config.fallback_repo_paths)?;
        packages
            .entry(pkg_id.clone())
            .or_default()
            .push(commit.clone());
        if closure.is_root(commit) {
            root_packages.insert(pkg_id);
        }
    }

    // checkout only root packages (deps get flattened into capsules, not checked out separately)
    for (pkg_id, commits) in &packages {
        // skip non-root packages - they're just deps that get flattened
        if !root_packages.contains(pkg_id) {
            continue;
        }
        let short_hash = &pkg_id.manifest_hash[..8.min(pkg_id.manifest_hash.len())];
        let pkg_subpath = format!("{}/{}", pkg_id.path, short_hash);
        let physical_pkg_dir = physical_nex_pkg.join(&pkg_id.path).join(short_hash);
        let logical_pkg_dir = logical_nex_pkg.join(&pkg_id.path).join(short_hash);

        // check logical path (visible through overlay) to see if already installed
        if logical_pkg_dir.exists() {
            println!("  {} already materialized", pkg_id.path);
            continue;
        }

        println!(
            "  Checking out {} ({} output{}) -> /nex/pkg/{}",
            pkg_id.path,
            commits.len(),
            if commits.len() > 1 { "s" } else { "" },
            pkg_subpath
        );
        fs::create_dir_all(&physical_pkg_dir)?;

        // checkout each commit (output) into the physical directory using --union
        for commit in commits {
            checkout_commit_flat(
                &config.repo_path,
                commit,
                &physical_pkg_dir,
                &config.fallback_repo_paths,
            )?;
        }

        // write .nex-app-root sentinel with all commits
        let sentinel = physical_pkg_dir.join(".nex-app-root");
        fs::write(&sentinel, commits.join("\n") + "\n")?;
    }

    // flatten runtime dependencies into root packages' lib/ directories
    // use physical path for writing but logical for checking existing
    if let Some(idx) = manifest_index {
        flatten_all_capsules_split(
            &config.repo_path,
            &physical_nex_pkg,
            &logical_nex_pkg,
            &packages,
            &root_packages,
            idx,
            &config.fallback_repo_paths,
        )?;
    }

    // create symlink forest in env (only for root packages)
    create_symlink_forest_split(
        config,
        &physical_nex_pkg,
        &physical_nex_env,
        &packages,
        &root_packages,
        result,
    )?;

    Ok(())
}

/// Flatten runtime dependencies into root package capsules using precomputed deps.
/// Uses physical path for writing, logical path for checking existing.
fn flatten_all_capsules_split(
    repo_path: &str,
    physical_nex_pkg: &Path,
    logical_nex_pkg: &Path,
    packages: &BTreeMap<PackageId, Vec<String>>,
    root_packages: &std::collections::HashSet<PackageId>,
    manifest_index: &ManifestIndex,
    fallback_repos: &[PathBuf],
) -> io::Result<()> {
    // flatten only root package capsules (deps don't need their own checkout)
    for (pkg_id, commits) in packages {
        // skip non-root packages
        if !root_packages.contains(pkg_id) {
            continue;
        }

        let short_hash = &pkg_id.manifest_hash[..8.min(pkg_id.manifest_hash.len())];
        let physical_pkg_dir = physical_nex_pkg.join(&pkg_id.path).join(short_hash);
        let logical_pkg_dir = logical_nex_pkg.join(&pkg_id.path).join(short_hash);

        // check logical path (visible through overlay)
        if !logical_pkg_dir.exists() && !physical_pkg_dir.exists() {
            continue;
        }

        // flatten using precomputed deps from first commit (all share same manifest)
        if let Some(commit) = commits.first() {
            let flattened_count = flatten_capsule_precomputed(
                repo_path,
                &physical_pkg_dir,
                commit,
                manifest_index,
                fallback_repos,
            )?;
            if flattened_count > 0 {
                println!(
                    "  Flattened {} libs into {}/{}",
                    flattened_count, pkg_id.path, short_hash
                );
            }
        }
    }

    Ok(())
}

/// Create symlink forest in /nex/env pointing to package files.
/// Writes symlinks to physical paths but targets are relative to logical root.
fn create_symlink_forest_split(
    config: &MaterializeConfig,
    physical_nex_pkg: &Path,
    physical_nex_env: &Path,
    packages: &BTreeMap<PackageId, Vec<String>>,
    root_packages: &std::collections::HashSet<PackageId>,
    result: &mut MaterializeResult,
) -> io::Result<()> {
    // logical paths - for checking what exists through overlay
    let logical_nex_pkg = config.target_dir.join("nex/pkg");
    let logical_nex_env = config.target_dir.join("nex/env");

    // dirs to symlink from packages
    let link_dirs = ["bin", "lib", "lib64", "sbin", "share", "include"];

    // only create symlinks for root packages
    for pkg_id in packages.keys() {
        if !root_packages.contains(pkg_id) {
            continue;
        }
        let short_hash = &pkg_id.manifest_hash[..8.min(pkg_id.manifest_hash.len())];
        // use physical path for reading (newly installed files)
        let physical_pkg_dir = physical_nex_pkg.join(&pkg_id.path).join(short_hash);
        // also check logical path (visible through overlay for already installed)
        let logical_pkg_dir = logical_nex_pkg.join(&pkg_id.path).join(short_hash);

        // determine which pkg_dir to read from (prefer logical if exists, else physical)
        let pkg_dir = if logical_pkg_dir.exists() {
            &logical_pkg_dir
        } else if physical_pkg_dir.exists() {
            &physical_pkg_dir
        } else {
            continue;
        };

        for subdir in &link_dirs {
            // check for usr/<dir> first, then top-level <dir>
            let src_paths = [pkg_dir.join("usr").join(subdir), pkg_dir.join(subdir)];

            for src in &src_paths {
                if !src.exists() {
                    continue;
                }

                // write symlinks to physical location
                let physical_env_subdir = physical_nex_env.join(subdir);
                // check existing in logical location (through overlay)
                let logical_env_subdir = logical_nex_env.join(subdir);
                fs::create_dir_all(&physical_env_subdir)?;

                // create symlinks with physical write path but logical check path
                create_symlinks_split(
                    src,
                    &physical_env_subdir,
                    &logical_env_subdir,
                    pkg_dir,
                    result,
                )?;
            }
        }
    }

    Ok(())
}

/// Recursively create symlinks from source to target directory (split physical/logical).
fn create_symlinks_split(
    src: &Path,
    physical_env_dir: &Path,
    logical_env_dir: &Path,
    pkg_root: &Path,
    result: &mut MaterializeResult,
) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let physical_target = physical_env_dir.join(&name);
        let logical_target = logical_env_dir.join(&name);

        if path.is_dir() {
            // recurse into subdirectories
            fs::create_dir_all(&physical_target)?;
            create_symlinks_split(&path, &physical_target, &logical_target, pkg_root, result)?;
        } else {
            // check if file already exists in logical view (through overlay)
            if logical_target.exists() || logical_target.symlink_metadata().is_ok() {
                result.warn(format!(
                    "Skipping {}: already exists",
                    logical_target.display()
                ));
                continue;
            }

            // also check physical location (in case we just wrote it)
            if physical_target.exists() || physical_target.symlink_metadata().is_ok() {
                result.warn(format!(
                    "Skipping {}: already exists in staging",
                    physical_target.display()
                ));
                continue;
            }

            // calculate relative path - this will work from the logical view
            let relative_target =
                diff_paths(&path, logical_env_dir).unwrap_or_else(|| path.clone());

            // write symlink to physical location
            std::os::unix::fs::symlink(&relative_target, &physical_target)?;
            result.symlinks_created.push(logical_target);
        }
    }

    Ok(())
}

/// Get the short hash (first 8 chars) of a commit.
fn get_commit_short_hash(
    repo_path: &str,
    commit: &str,
    fallback_repos: &[PathBuf],
) -> io::Result<String> {
    let store = Store::open_with_fallback_chain(repo_path, fallback_repos)?;
    let commit_id = store.resolve_ref(commit)?;
    Ok(commit_id[..8.min(commit_id.len())].to_string())
}

/// Get the manifest hash from a commit's metadata.
fn get_manifest_hash(
    repo_path: &str,
    commit: &str,
    fallback_repos: &[PathBuf],
) -> io::Result<String> {
    let store = Store::open_with_fallback_chain(repo_path, fallback_repos)?;
    match store.get_metadata(commit, "nex.manifest.hash")? {
        Some(hash) if !hash.is_empty() => Ok(hash),
        _ => get_commit_short_hash(repo_path, commit, fallback_repos),
    }
}

/// Extract package path from ref (e.g., "x86_64/pkg/core/init/systemd/1.0/outputs/bin" -> "core/init/systemd/1.0").
fn get_package_path(
    repo_path: &str,
    commit: &str,
    fallback_repos: &[PathBuf],
) -> io::Result<String> {
    let store = Store::open_with_fallback_chain(repo_path, fallback_repos)?;
    // try to get ref-binding metadata
    match store.get_metadata(commit, "nex.ref-binding")? {
        Some(binding) => {
            // binding looks like: ['x86_64/pkg/core/init/systemd/1.0/outputs/bin']
            // extract the ref and parse it
            let ref_str = binding
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim_matches('\'')
                .trim_matches('"');
            parse_package_path_from_ref(ref_str)
        }
        None => parse_package_path_from_ref(commit),
    }
}

/// Parse package path from a ref string.
fn parse_package_path_from_ref(ref_str: &str) -> io::Result<String> {
    // ref format: x86_64/pkg/<namespace>/<slug>/<version>/outputs/<output>
    // or: x86_64/pkg/<namespace>/<slug>/<version>/bundles/<bundle>
    // we want: <namespace>/<slug>/<version>

    let parts: Vec<&str> = ref_str.split('/').collect();

    // find "pkg" and extract everything between "pkg" and "outputs"/"bundles"
    if let Some(pkg_idx) = parts.iter().position(|&p| p == "pkg") {
        // find "outputs" or "bundles"
        let end_idx = parts
            .iter()
            .position(|&p| p == "outputs" || p == "bundles")
            .unwrap_or(parts.len());

        if end_idx > pkg_idx + 1 {
            let path_parts = &parts[pkg_idx + 1..end_idx];
            return Ok(path_parts.join("/"));
        }
    }

    // fallback: use the whole ref as the path
    Ok(ref_str.replace('/', "_"))
}

/// Get the package identity for a commit.
fn get_package_id(
    repo_path: &str,
    commit: &str,
    fallback_repos: &[PathBuf],
) -> io::Result<PackageId> {
    let manifest_hash = get_manifest_hash(repo_path, commit, fallback_repos)?;
    let path = get_package_path(repo_path, commit, fallback_repos)?;

    Ok(PackageId {
        path,
        manifest_hash,
    })
}

/// Checkout specific files from a commit to a target directory.
/// For symlinks, also copies the target file to ensure the symlink is valid.
pub fn checkout_files(
    repo_path: &str,
    commit: &str,
    files: &[String],
    target_dir: &Path,
    fallback_repos: &[PathBuf],
) -> io::Result<()> {
    // checkout to temp dir on same filesystem as target to allow hardlinks
    let temp = TempDir::new_in(target_dir)?;
    checkout_commit_flat(repo_path, commit, temp.path(), fallback_repos)?;

    for file in files {
        let src = temp.path().join(file.trim_start_matches('/'));
        if !src.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("File {} not found in commit {}", file, commit),
            ));
        }

        let dst = target_dir.join(file.trim_start_matches('/'));
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }

        // handle symlinks: copy both the symlink AND its target
        if src.symlink_metadata()?.file_type().is_symlink() {
            let link_target = fs::read_link(&src)?;

            // copy the symlink target first (if it's a relative path in same dir)
            if !link_target.is_absolute() {
                let target_src = src.parent().unwrap().join(&link_target);
                if target_src.exists() && !target_src.symlink_metadata()?.file_type().is_symlink() {
                    let target_dst = dst.parent().unwrap().join(&link_target);
                    if !target_dst.exists() {
                        fs::copy(&target_src, &target_dst)?;
                    }
                }
            }

            // then create the symlink
            if dst.exists() || dst.symlink_metadata().is_ok() {
                fs::remove_file(&dst)?;
            }
            std::os::unix::fs::symlink(&link_target, &dst)?;
        } else {
            // skip copy if dst already exists with same inode (already hardlinked from earlier checkout)
            use std::os::unix::fs::MetadataExt;
            let src_ino = fs::metadata(&src).map(|m| m.ino()).ok();
            let dst_ino = if dst.exists() { fs::metadata(&dst).map(|m| m.ino()).ok() } else { None };
            if src_ino.is_some() && src_ino == dst_ino {
                // same file via hardlink, skip copy to avoid corruption
                continue;
            }
            fs::copy(&src, &dst)?;
        }
    }

    Ok(())
}
