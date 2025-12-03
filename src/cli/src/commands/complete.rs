//! ref completion for shell auto-completion.

use std::io;
use std::path::Path;

use clap::Args;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::manifest::{load_manifest, ManifestData};
use crate::repo::{detect_context, NexContext};
use crate::store::Store;

#[derive(Args)]
pub struct CompleteArgs {
    /// partial ref prefix to complete (empty for all refs)
    #[clap(default_value = "")]
    pub prefix: String,

    /// don't check cache status (faster, no [cached] indicator)
    #[clap(long)]
    pub no_cache_check: bool,
}

pub fn run(args: &CompleteArgs) -> io::Result<()> {
    // detect context to get manifest directories and repo path
    let ctx = detect_context(false)?;

    // collect all manifest directories to search
    let manifest_dirs = collect_manifest_dirs(&ctx);
    if manifest_dirs.is_empty() {
        return Ok(());
    }

    // open store for cache checking (if enabled)
    let store = if !args.no_cache_check {
        Store::open_with_fallback_chain(&ctx.repo_path, &ctx.fallback_repos).ok()
    } else {
        None
    };

    // generate all possible refs from manifests
    for manifest_dir in &manifest_dirs {
        generate_refs_from_dir(manifest_dir, &args.prefix, store.as_ref())?;
    }

    Ok(())
}

fn collect_manifest_dirs(ctx: &NexContext) -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();

    // user worktree
    if let Some(ref manifests_path) = ctx.manifests_path {
        let pkg_dir = manifests_path.join("pkg");
        if pkg_dir.exists() {
            dirs.push(pkg_dir);
        }
    }

    // system manifests
    if Path::new("/nex/db/pkg").exists() {
        dirs.push(std::path::PathBuf::from("/nex/db/pkg"));
    }

    // local development
    if Path::new("pkg").exists() {
        dirs.push(std::path::PathBuf::from("pkg"));
    }

    dirs
}

fn generate_refs_from_dir(
    manifest_dir: &Path,
    prefix: &str,
    store: Option<&Store>,
) -> io::Result<()> {
    for entry in WalkDir::new(manifest_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // only process yaml files
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("yaml") && ext != Some("yml") {
            continue;
        }

        // skip non-files
        if !path.is_file() {
            continue;
        }

        // try to load and generate refs
        if let Ok(refs) = generate_refs_for_manifest(path, manifest_dir, store) {
            for (ref_str, cached) in refs {
                if ref_str.starts_with(prefix) {
                    if cached {
                        println!("{} [cached]", ref_str);
                    } else {
                        println!("{}", ref_str);
                    }
                }
            }
        }
    }

    Ok(())
}

fn generate_refs_for_manifest(
    manifest_path: &Path,
    manifest_dir: &Path,
    store: Option<&Store>,
) -> io::Result<Vec<(String, bool)>> {
    let manifest_data = load_manifest(&manifest_path.to_string_lossy())?;

    let manifest = match manifest_data {
        ManifestData::Package(m) => m,
        ManifestData::System(_) => return Ok(vec![]), // skip system manifests
    };

    // get namespace from directory structure
    let rel_path = manifest_path
        .strip_prefix(manifest_dir)
        .unwrap_or(manifest_path);
    let namespace = rel_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    if namespace.is_empty() {
        return Ok(vec![]);
    }

    let slug = &manifest.package.slug;
    let version = &manifest.package.version;

    // compute manifest hash for staleness check
    let manifest_content = std::fs::read_to_string(manifest_path)?;
    let manifest_hash = format!("{:x}", Sha256::digest(manifest_content.as_bytes()));

    let mut refs = Vec::new();

    // generate output refs
    for output_name in manifest.outputs.keys() {
        let ref_str = format!(
            "x86_64/pkg/{}/{}/{}/outputs/{}",
            namespace, slug, version, output_name
        );
        let cached = check_cached(store, &ref_str, &manifest_hash);
        refs.push((ref_str, cached));
    }

    // generate bundle refs
    for bundle_name in manifest.bundles.keys() {
        let ref_str = format!(
            "x86_64/pkg/{}/{}/{}/bundles/{}",
            namespace, slug, version, bundle_name
        );
        let cached = check_cached(store, &ref_str, &manifest_hash);
        refs.push((ref_str, cached));
    }

    // generate files ref
    let files_ref = format!("x86_64/pkg/{}/{}/{}/files", namespace, slug, version);
    let files_cached = check_cached(store, &files_ref, &manifest_hash);
    refs.push((files_ref, files_cached));

    Ok(refs)
}

fn check_cached(store: Option<&Store>, ref_str: &str, expected_hash: &str) -> bool {
    let store = match store {
        Some(s) => s,
        None => return false,
    };

    // check if ref exists
    if store.resolve_ref(ref_str).is_err() {
        return false;
    }

    // check manifest hash matches
    match store.get_metadata(ref_str, "nex.manifest.hash") {
        Ok(Some(stored_hash)) => stored_hash == expected_hash,
        _ => false,
    }
}
