use clap::Args;
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use super::build::BuildOpts;
use super::stage::{is_staged, mount_nex_overlays, staging_upper_dir};
use super::state::InstalledState;
use crate::build;
use crate::manifest::{load_manifest, ManifestData};
use crate::materializer::{materialize, MaterializeConfig, MaterializeMode, MaterializeRequest};
use crate::refs::PackageRef;
use crate::repo::{detect_context, ensure_user_dirs, resolve_repo_path};
use crate::store::Store;

#[derive(Args)]
pub struct InstallArgs {
    /// Manifest path (e.g., /nex/db/pkg/cli/editors/neovim.yaml)
    pub manifest: String,

    /// Target to install: bundles/full (default), outputs/bin, files/usr/lib/x.so
    #[clap(default_value = "bundles/full")]
    pub target: String,

    /// Repository path (auto-detected if not specified)
    #[clap(long)]
    pub repo: Option<String>,

    /// Commit immediately after installing (stage, install, commit in one step)
    #[clap(long)]
    pub commit: bool,

    /// Skip staging check (for use during system assembly)
    #[clap(long, hide = true)]
    pub no_stage_check: bool,

    /// Use flat layout instead of /nex/pkg structure
    #[clap(long)]
    pub flat: bool,

    /// Skip runtime dependency resolution
    #[clap(long)]
    pub no_deps: bool,

    /// Show what would be installed without actually installing
    #[clap(long)]
    pub dry_run: bool,

    /// Install to the system-wide location (requires root)
    /// Without this flag, packages are installed to the user's local environment
    #[clap(long)]
    pub system: bool,
}

pub fn run(args: &InstallArgs) -> io::Result<()> {
    // load manifest
    let manifest_path = Path::new(&args.manifest);
    if !manifest_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Manifest not found: {}", args.manifest),
        ));
    }

    let manifest_data = load_manifest(&args.manifest)?;
    let manifest = match manifest_data {
        ManifestData::Package(m) => m,
        ManifestData::System(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Cannot install a system manifest directly",
            ))
        }
    };

    // construct package ref from manifest + target
    let package_ref = format!(
        "x86_64/{}/{}/{}/{}",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version,
        args.target
    );

    // detect context: user install vs system install
    let ctx = detect_context(args.system)?;

    // for user installs, ensure user directories exist (check for config.toml, the zub repo marker)
    if !ctx.is_system && !ctx.repo_path.join("config.toml").exists() {
        ensure_user_dirs(&ctx)?;
        println!(
            "Created user environment at {}",
            ctx.repo_path.parent().unwrap_or(&ctx.repo_path).display()
        );
    }

    // use explicit repo path if provided, otherwise use context-determined path
    let repo_path = if let Some(ref r) = args.repo {
        resolve_repo_path(Some(r))?
    } else {
        ctx.repo_path.to_string_lossy().to_string()
    };

    // derive manifest db path from manifest location for dependency resolution
    let manifest_db_paths = derive_manifest_db_path(manifest_path)
        .map(|p| vec![p])
        .unwrap_or_else(|| ctx.manifest_dirs.clone());

    // system installs require staging mode (unless build-time or --no-stage-check)
    if ctx.is_system && ctx.needs_staging {
        // check staging mode unless --commit or --no-stage-check
        if !args.commit && !args.no_stage_check && !is_staged() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Not in staging mode. Use 'nex stage' first, or use 'nex install --commit <pkg>' for atomic install.",
            ));
        }

        // if --commit, enter staging mode first
        if args.commit && !is_staged() {
            println!("Entering staging mode for atomic install...");
            super::stage::run(&super::stage::StageArgs { force: false })?;
        }
    }

    // load current state (context-aware)
    let mut state = InstalledState::load_for_context(&ctx).unwrap_or_default();

    // parse the package ref (for checksum lookup etc)
    let pkg_ref = PackageRef::parse(&package_ref).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid package ref '{}': {}", package_ref, e),
        )
    })?;

    // validate target exists in manifest
    validate_target(&manifest, &args.target)?;

    // compute manifest hash for staleness check
    let manifest_content = fs::read_to_string(&args.manifest)?;
    let manifest_hash = format!("{:x}", Sha256::digest(manifest_content.as_bytes()));

    // check if ref exists and is fresh in cache
    let store = Store::open_with_fallback_chain(&repo_path, &ctx.fallback_repos).ok();
    let is_cached = check_cache_freshness(store.as_ref(), &package_ref, &manifest_hash);

    // build if not cached or stale
    if !is_cached {
        println!("Building {}...", package_ref);
        build_package_to_user_repo(
            &repo_path,
            &ctx.fallback_repos,
            manifest_path,
        )?;
    }

    println!("Installing {}...", package_ref);

    // get checksum from zub (now guaranteed to exist after build)
    let checksum = get_commit_short_hash_with_fallback(
        &repo_path,
        ctx.fallback_repo().map(|p| p.as_path()),
        &package_ref,
    )?;

    let pkg_info = PackageInfo {
        namespace: pkg_ref.namespace.clone(),
        slug: pkg_ref.slug.clone(),
        version: pkg_ref.version.clone(),
        checksum,
    };

    // check if this exact version is already installed
    if state.is_version_installed(
        &pkg_info.namespace,
        &pkg_info.slug,
        &pkg_info.version,
        &pkg_info.checksum,
    ) {
        println!(
            "  Version {}/{} already installed",
            pkg_info.version, pkg_info.checksum
        );

        // check if it's the current version
        let current = state.get_current_version(&pkg_info.namespace, &pkg_info.slug);
        let version_key = format!("{}/{}", pkg_info.version, pkg_info.checksum);

        if current == Some(version_key.as_str()) {
            println!("  Already the current version, nothing to do.");
            return Ok(());
        } else {
            println!("  Not the current version. Use 'nex switch' to activate it.");
            return Ok(());
        }
    }

    // check if another version is installed (for symlink decision)
    let existing_version_count = state.version_count(&pkg_info.namespace, &pkg_info.slug);
    let has_existing_version = existing_version_count > 0;

    // determine target directory and mode based on context
    let (target_dir, nex_pkg_dir, nex_env_dir, mode, pkg_override, env_override) = if args.flat {
        (
            "/".to_string(),
            "/nex/pkg".to_string(),
            "/nex/env".to_string(),
            MaterializeMode::Flat,
            None,
            None,
        )
    } else if ctx.is_system {
        (
            "/".to_string(),
            "/nex/pkg".to_string(),
            "/nex/env".to_string(),
            MaterializeMode::Nex,
            None,
            None,
        )
    } else {
        // user install: override pkg/env directories to user's paths
        (
            "/".to_string(),
            ctx.pkg_path.to_string_lossy().to_string(),
            ctx.env_path.to_string_lossy().to_string(),
            MaterializeMode::Nex,
            Some(ctx.pkg_path.clone()),
            Some(ctx.env_path.clone()),
        )
    };

    // materialize the package and its dependencies
    // system installs when staged write to overlay upper dir (bypasses OverlayFS device boundary)
    // user installs write directly to user's directories (no staging needed)
    let physical_root = if ctx.needs_staging && is_staged() {
        staging_upper_dir().map(Into::into)
    } else {
        None
    };

    let config = MaterializeConfig {
        repo_path: repo_path.clone(),
        target_dir: target_dir.into(),
        physical_root,
        mode,
        resolve_deps: !args.no_deps,
        manifest_db_paths: manifest_db_paths.clone(),
        fallback_repo_paths: ctx.fallback_repos.clone(),
        pkg_dir_override: pkg_override,
        env_dir_override: env_override,
        ..Default::default()
    };

    let requests = vec![MaterializeRequest::Bundle {
        commit: package_ref.clone(),
    }];

    let result = materialize(&config, &requests)?;

    // system installs: mount overlays after checkout
    if ctx.is_system && ctx.needs_staging && is_staged() {
        mount_nex_overlays()?;
    }

    // report results
    if result.closure.has_unresolved() {
        println!("\nWarning: Some dependencies could not be resolved:");
        for (dep, reasons) in &result.closure.unresolved {
            println!("  - {} ({} references)", dep, reasons.len());
        }
    }

    // get binaries provided by this package (from the checkout)
    let binaries = if mode == MaterializeMode::Nex {
        let pkg_dir = format!(
            "{}/{}/{}/{}/{}",
            nex_pkg_dir, pkg_info.namespace, pkg_info.slug, pkg_info.version, pkg_info.checksum
        );
        get_binaries_from_dir(&pkg_dir)?
    } else {
        vec![]
    };

    // symlink decision: only create symlinks if no other version exists and we're in nex mode
    let should_create_symlinks = !has_existing_version && mode == MaterializeMode::Nex;

    if should_create_symlinks && !binaries.is_empty() {
        // symlink to env/default/bin for user installs, /usr/bin for system installs
        let bin_dir = if ctx.is_system {
            "/usr/bin".to_string()
        } else {
            format!("{}/default/bin", nex_env_dir)
        };
        fs::create_dir_all(&bin_dir)?;

        for binary in &binaries {
            let dst = format!("{}/{}", bin_dir, binary);
            // calculate relative symlink target
            let relative_target = if ctx.is_system {
                format!(
                    "../../nex/pkg/{}/{}/{}/{}/usr/bin/{}",
                    pkg_info.namespace, pkg_info.slug, pkg_info.version, pkg_info.checksum, binary
                )
            } else {
                // from env/default/bin to pkg/<ns>/<slug>/<ver>/<hash>/usr/bin
                format!(
                    "../../../pkg/{}/{}/{}/{}/usr/bin/{}",
                    pkg_info.namespace, pkg_info.slug, pkg_info.version, pkg_info.checksum, binary
                )
            };

            // remove existing symlink if present
            if Path::new(&dst).exists() || Path::new(&dst).is_symlink() {
                fs::remove_file(&dst)?;
            }
            symlink(&relative_target, &dst)?;
            println!("  Linked {} -> {}", binary, relative_target);
        }
        println!(
            "Installed {}/{} {} (set as current)",
            pkg_info.namespace, pkg_info.slug, pkg_info.version
        );
    } else if has_existing_version {
        println!("  Installed alongside existing version(s)");
        println!(
            "  Symlinks preserved. Use 'nex switch {}/{} {}' to activate this version.",
            pkg_info.namespace, pkg_info.slug, pkg_info.version
        );
    } else {
        println!(
            "Installed {}/{} {}",
            pkg_info.namespace, pkg_info.slug, pkg_info.version
        );
    }

    // record the install in state
    state.record_install(
        &pkg_info.namespace,
        &pkg_info.slug,
        &pkg_info.version,
        &pkg_info.checksum,
        binaries,
        &package_ref,
        should_create_symlinks,
    );

    // save state (context-aware)
    state.save_for_context(&ctx)?;

    // if --commit (system install only), commit the changes
    if args.commit && ctx.is_system {
        println!("\nCommitting changes...");
        super::commit::run(&super::commit::CommitArgs {
            message: Some(format!("Install {}/{}", pkg_info.slug, pkg_info.version)),
        })?;
    }

    Ok(())
}

struct PackageInfo {
    namespace: String,
    slug: String,
    version: String,
    checksum: String,
}

/// validate that target (bundles/full, outputs/bin, files/...) exists in manifest
fn validate_target(manifest: &crate::manifest::Manifest, target: &str) -> io::Result<()> {
    let parts: Vec<&str> = target.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid target '{}': expected format like bundles/full or outputs/bin", target),
        ));
    }

    let (kind, name) = (parts[0], parts[1]);
    match kind {
        "bundles" => {
            if !manifest.bundles.contains_key(name) {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "Bundle '{}' not found in manifest (available: {:?})",
                        name,
                        manifest.bundles.keys().collect::<Vec<_>>()
                    ),
                ));
            }
        }
        "outputs" => {
            if !manifest.outputs.contains_key(name) {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "Output '{}' not found in manifest (available: {:?})",
                        name,
                        manifest.outputs.keys().collect::<Vec<_>>()
                    ),
                ));
            }
        }
        "files" => {
            // files are always valid if path makes sense
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid target kind '{}': expected bundles, outputs, or files", kind),
            ));
        }
    }
    Ok(())
}

/// derive manifest db path from a manifest file path
/// e.g., /nex/db/pkg/cli/editors/neovim.yaml -> /nex/db/pkg
fn derive_manifest_db_path(manifest_path: &Path) -> Option<PathBuf> {
    let path_str = manifest_path.to_string_lossy();
    // find /pkg/ in the path and return everything up to and including it
    if let Some(idx) = path_str.find("/pkg/") {
        return Some(PathBuf::from(&path_str[..idx + 4]));
    }
    // fallback: if path starts with pkg/, use pkg/
    if path_str.starts_with("pkg/") {
        return Some(PathBuf::from("pkg"));
    }
    None
}

/// check if ref is cached and manifest hash matches (not stale)
fn check_cache_freshness(store: Option<&Store>, ref_str: &str, expected_hash: &str) -> bool {
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

fn get_commit_short_hash_with_fallback(
    repo: &str,
    fallback: Option<&Path>,
    ref_path: &str,
) -> io::Result<String> {
    // use manifest hash (shared by all outputs of a build) - must match checkout.rs
    let store = Store::open_with_fallback(repo, fallback)?;

    // try to get manifest hash from metadata first
    if let Ok(Some(manifest_hash)) = store.get_metadata(ref_path, "nex.manifest.hash") {
        return Ok(manifest_hash[..8.min(manifest_hash.len())].to_string());
    }

    // fallback to commit ID if no manifest hash
    match store.resolve_ref(ref_path) {
        Ok(id) => Ok(id[..8.min(id.len())].to_string()),
        Err(_) => {
            // fallback to hash of ref path
            use sha2::{Digest, Sha256};
            let hash = hex::encode(Sha256::digest(ref_path.as_bytes()));
            Ok(hash[..8].to_string())
        }
    }
}

fn get_binaries_from_dir(pkg_dir: &str) -> io::Result<Vec<String>> {
    let bin_dir = Path::new(pkg_dir).join("usr/bin");
    if !bin_dir.exists() {
        return Ok(vec![]);
    }

    let mut binaries = Vec::new();
    for entry in fs::read_dir(&bin_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() || entry.file_type()?.is_symlink() {
            if let Some(name) = entry.file_name().to_str() {
                binaries.push(name.to_string());
            }
        }
    }

    Ok(binaries)
}

/// build a package and commit to repo (calls build library directly, no subprocess)
fn build_package_to_user_repo(
    repo_path: &str,
    fallback_repos: &[PathBuf],
    manifest_path: &Path,
) -> io::Result<()> {
    println!("Building from manifest: {}", manifest_path.display());

    let opts = BuildOpts {
        repo_path: repo_path.to_string(),
        manifest_file: manifest_path.to_string_lossy().to_string(),
        check: false,
        update_checksum: false,
        compute_deps: false,
        runtime_deps_verbose: false,
        refresh_metadata: false,
        force: false,
        build_dir: None,
        generate_outputs: false,
        fallback_repos: fallback_repos
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
        verbose: false,
        record_profile: false,
        no_progress: false,
        dry_run: false,
        add_checksums: false,
        show_dep_paths: false,
        trace_dependency: None,
        multi_progress: None,
    };

    build::build_single(&opts)
}
