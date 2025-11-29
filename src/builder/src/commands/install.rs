use clap::Args;
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::stage::{is_staged, mount_nex_overlays, staging_upper_dir};
use super::state::InstalledState;
use crate::materializer::{materialize, MaterializeConfig, MaterializeMode, MaterializeRequest};
use crate::ostree_native::OstreeRepo;
use crate::repo::{detect_context, detect_manifest_dir, ensure_user_dirs, resolve_repo_path, NexContext};

#[derive(Args)]
pub struct InstallArgs {
    /// Package to install (e.g., "bash", "cli/shells/bash", or full ref)
    pub package: String,

    /// OSTree repository path (auto-detected if not specified)
    #[clap(long)]
    pub repo: Option<String>,

    /// Install specific version
    #[clap(long)]
    pub version: Option<String>,

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
    // detect context: user install vs system install
    let ctx = detect_context(args.system)?;

    // for user installs, ensure user directories exist
    if !ctx.is_system && !ctx.repo_path.exists() {
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

    // find the bundle or output ref for the package
    // for user installs, we look in the system repo via fallback
    // if not found and in user context, try to auto-build
    let package_ref = match find_package_ref_with_fallback(
        &repo_path,
        ctx.fallback_repo.as_deref(),
        &args.package,
        args.version.as_deref(),
    ) {
        Ok(r) => r,
        Err(e) if e.kind() == io::ErrorKind::NotFound && !ctx.is_system => {
            // package not found - try to auto-build in user context
            println!(
                "Package '{}' not found in repos, attempting build...",
                args.package
            );

            let manifest_path =
                find_manifest_for_package(&ctx, &args.package, args.version.as_deref())?;
            build_package_to_user_repo(&ctx, &manifest_path)?;

            // retry finding the package
            find_package_ref_with_fallback(
                &repo_path,
                ctx.fallback_repo.as_deref(),
                &args.package,
                args.version.as_deref(),
            )?
        }
        Err(e) => return Err(e),
    };
    println!("Installing {}...", package_ref);

    // parse the ref to get package info
    let pkg_info = parse_package_ref_with_fallback(
        &repo_path,
        ctx.fallback_repo.as_deref(),
        &package_ref,
    )?;

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
        ("/".to_string(), "/nex/pkg".to_string(), "/nex/env".to_string(), MaterializeMode::Flat, None, None)
    } else if ctx.is_system {
        ("/".to_string(), "/nex/pkg".to_string(), "/nex/env".to_string(), MaterializeMode::Nex, None, None)
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
        manifest_db_path: detect_manifest_dir().map(Into::into),
        fallback_repo_path: ctx.fallback_repo.clone(),
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

fn find_package_ref_with_fallback(
    repo: &str,
    fallback: Option<&Path>,
    query: &str,
    version: Option<&str>,
) -> io::Result<String> {
    let ostree_repo = OstreeRepo::open_with_fallback(repo, fallback)?;
    let all_refs = ostree_repo.refs(None)?;
    let query_parts: Vec<&str> = query.split('/').collect();

    // prefer bundles/full, then bundles/*, then outputs/bin
    let priorities = ["/bundles/full", "/bundles/", "/outputs/bin"];

    for priority in &priorities {
        for ref_name in &all_refs {
            if !ref_name.contains(priority) {
                continue;
            }

            let parts: Vec<&str> = ref_name.split('/').collect();
            if parts.len() < 6 || parts[0] != "x86_64" || parts[1] != "pkg" {
                continue;
            }

            let boundary = parts
                .iter()
                .position(|p| *p == "bundles" || *p == "outputs");

            let boundary = match boundary {
                Some(b) => b,
                None => continue,
            };

            if boundary < 4 {
                continue;
            }

            let namespace = parts[2..boundary - 2].join("/");
            let slug = parts[boundary - 2];
            let ver = parts[boundary - 1];

            // filter by version if specified
            if let Some(v) = version {
                if ver != v {
                    continue;
                }
            }

            // match query
            let matched = if query_parts.len() == 1 {
                slug == query_parts[0]
            } else {
                let full_path = format!("{}/{}", namespace, slug);
                full_path.contains(query) || full_path.ends_with(query)
            };

            if matched {
                return Ok(ref_name.to_string());
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("No package found for '{}'. Build it first.", query),
    ))
}

fn parse_package_ref_with_fallback(
    repo: &str,
    fallback: Option<&Path>,
    ref_path: &str,
) -> io::Result<PackageInfo> {
    let parts: Vec<&str> = ref_path.split('/').collect();
    let boundary = parts
        .iter()
        .position(|p| *p == "bundles" || *p == "outputs")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid package ref"))?;

    if boundary < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid package ref format",
        ));
    }

    // get commit hash for checksum
    let checksum = get_commit_short_hash_with_fallback(repo, fallback, ref_path)?;

    Ok(PackageInfo {
        namespace: parts[2..boundary - 2].join("/"),
        slug: parts[boundary - 2].to_string(),
        version: parts[boundary - 1].to_string(),
        checksum,
    })
}

fn get_commit_short_hash_with_fallback(
    repo: &str,
    fallback: Option<&Path>,
    ref_path: &str,
) -> io::Result<String> {
    // use manifest hash (shared by all outputs of a build) - must match checkout.rs
    let ostree_repo = OstreeRepo::open_with_fallback(repo, fallback)?;

    // try to get manifest hash from metadata first
    if let Ok(Some(manifest_hash)) = ostree_repo.get_metadata(ref_path, "nex.manifest.hash") {
        return Ok(manifest_hash[..8.min(manifest_hash.len())].to_string());
    }

    // fallback to commit ID if no manifest hash
    match ostree_repo.resolve_ref(ref_path) {
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

/// find manifest file for a package query
fn find_manifest_for_package(
    ctx: &NexContext,
    query: &str,
    version: Option<&str>,
) -> io::Result<PathBuf> {
    // search order: user worktree -> /nex/db/pkg -> local pkg/
    let search_paths: Vec<PathBuf> = [
        ctx.manifests_path.clone(),
        Some(PathBuf::from("/nex/db/pkg")),
        Some(PathBuf::from("pkg")),
    ]
    .into_iter()
    .flatten()
    .filter(|p| p.exists())
    .collect();

    for base in &search_paths {
        if let Some(found) = search_manifest_in_dir(base, query, version)? {
            return Ok(found);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "No manifest found for '{}'. Add to your manifests worktree at {:?}.",
            query,
            ctx.manifests_path
        ),
    ))
}

/// search for manifest matching query in a directory
fn search_manifest_in_dir(
    base: &Path,
    query: &str,
    _version: Option<&str>,
) -> io::Result<Option<PathBuf>> {
    // query can be: "bash", "cli/shells/bash", full path, etc.
    let parts: Vec<&str> = query.split('/').collect();
    let slug = parts.last().unwrap_or(&query);

    // walk the directory looking for yaml files
    for entry in walkdir(base)? {
        let path = entry?;
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("yaml") && ext != Some("yml") {
            continue;
        }

        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        // match by slug (file stem matches query slug)
        if file_stem == *slug {
            return Ok(Some(path));
        }

        // also try matching the full path pattern
        let rel_path = path.strip_prefix(base).unwrap_or(&path);
        let rel_str = rel_path.to_string_lossy();
        if rel_str.contains(query) {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

/// recursively walk a directory yielding file paths
fn walkdir(base: &Path) -> io::Result<Vec<io::Result<PathBuf>>> {
    let mut results = Vec::new();
    walkdir_inner(base, &mut results)?;
    Ok(results)
}

fn walkdir_inner(dir: &Path, results: &mut Vec<io::Result<PathBuf>>) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walkdir_inner(&path, results)?;
        } else {
            results.push(Ok(path));
        }
    }

    Ok(())
}

/// build a package and commit to user's repo
fn build_package_to_user_repo(ctx: &NexContext, manifest_path: &Path) -> io::Result<()> {
    println!("Building from manifest: {}", manifest_path.display());

    // invoke nex build with user's repo path
    let output = Command::new(std::env::current_exe()?)
        .arg("build")
        .arg("--single")
        .arg("--repo")
        .arg(&ctx.repo_path)
        .arg(manifest_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Build failed:\nstdout: {}\nstderr: {}",
                stdout, stderr
            ),
        ));
    }

    // print build output
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        print!("{}", stdout);
    }

    Ok(())
}
