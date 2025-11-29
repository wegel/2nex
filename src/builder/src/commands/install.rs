use clap::Args;
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::Path;

use super::stage::{is_staged, mount_nex_overlays, staging_upper_dir};
use super::state::InstalledState;
use crate::materializer::{
    materialize, MaterializeConfig, MaterializeMode, MaterializeRequest,
};
use crate::ostree_native::OstreeRepo;
use crate::repo::{detect_manifest_dir, resolve_repo_path};

const NEX_PKG_DIR: &str = "/nex/pkg";
const USR_BIN_DIR: &str = "/usr/bin";

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
}

pub fn run(args: &InstallArgs) -> io::Result<()> {
    let repo_path = resolve_repo_path(args.repo.as_deref())?;

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

    // load current state
    let mut state = InstalledState::load().unwrap_or_default();

    // find the bundle or output ref for the package
    let package_ref = find_package_ref(&repo_path, &args.package, args.version.as_deref())?;
    println!("Installing {}...", package_ref);

    // parse the ref to get package info
    let pkg_info = parse_package_ref(&repo_path, &package_ref)?;

    // check if this exact version is already installed
    if state.is_version_installed(&pkg_info.namespace, &pkg_info.slug, &pkg_info.version, &pkg_info.checksum) {
        println!("  Version {}/{} already installed", pkg_info.version, pkg_info.checksum);

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

    // determine target directory and mode
    let (target_dir, mode) = if args.flat {
        ("/".to_string(), MaterializeMode::Flat)
    } else {
        ("/".to_string(), MaterializeMode::Nex)
    };

    // materialize the package and its dependencies
    // when staged, write to overlay upper dir (bypasses OverlayFS device boundary for hardlinks)
    let physical_root = staging_upper_dir().map(Into::into);
    let config = MaterializeConfig {
        repo_path: repo_path.clone(),
        target_dir: target_dir.into(),
        physical_root,
        mode,
        resolve_deps: !args.no_deps,
        manifest_db_path: detect_manifest_dir().map(Into::into),
        ..Default::default()
    };

    let requests = vec![MaterializeRequest::Bundle {
        commit: package_ref.clone(),
    }];

    let result = materialize(&config, &requests)?;

    // mount /nex/pkg and /nex/env overlays now that checkout is complete
    mount_nex_overlays()?;

    // report results
    if result.closure.has_unresolved() {
        println!("\nWarning: Some dependencies could not be resolved:");
        for (dep, reasons) in &result.closure.unresolved {
            println!("  - {} ({} references)", dep, reasons.len());
        }
    }

    // get binaries provided by this package (from the checkout)
    let binaries = if mode == MaterializeMode::Nex {
        // in nex mode, get binaries from /nex/pkg/<ns>/<slug>/<ver>/<hash>/usr/bin
        let pkg_dir = format!(
            "{}/{}/{}/{}/{}",
            NEX_PKG_DIR, pkg_info.namespace, pkg_info.slug, pkg_info.version, pkg_info.checksum
        );
        get_binaries_from_dir(&pkg_dir)?
    } else {
        vec![]
    };

    // symlink decision: only create symlinks if no other version exists and we're in nex mode
    let should_create_symlinks = !has_existing_version && mode == MaterializeMode::Nex;

    if should_create_symlinks && !binaries.is_empty() {
        for binary in &binaries {
            let dst = format!("{}/{}", USR_BIN_DIR, binary);
            // relative symlink: from /usr/bin/, ../../ reaches /, then nex/pkg/...
            let relative_target = format!(
                "../../nex/pkg/{}/{}/{}/{}/usr/bin/{}",
                pkg_info.namespace, pkg_info.slug, pkg_info.version, pkg_info.checksum, binary
            );

            // ensure /usr/bin exists
            fs::create_dir_all(USR_BIN_DIR)?;

            // remove existing symlink if present (could be from different package)
            if Path::new(&dst).exists() || Path::new(&dst).is_symlink() {
                fs::remove_file(&dst)?;
            }
            symlink(&relative_target, &dst)?;
            println!("  Linked {} -> {}", binary, relative_target);
        }
        println!("Installed {}/{} {} (set as current)", pkg_info.namespace, pkg_info.slug, pkg_info.version);
    } else if has_existing_version {
        println!("  Installed alongside existing version(s)");
        println!("  Symlinks preserved. Use 'nex switch {}/{} {}' to activate this version.",
                 pkg_info.namespace, pkg_info.slug, pkg_info.version);
    } else {
        println!("Installed {}/{} {}", pkg_info.namespace, pkg_info.slug, pkg_info.version);
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

    // save state
    state.save()?;

    // if --commit, commit the changes
    if args.commit {
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

fn find_package_ref(repo: &str, query: &str, version: Option<&str>) -> io::Result<String> {
    let ostree_repo = OstreeRepo::open(repo)?;
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

fn parse_package_ref(repo: &str, ref_path: &str) -> io::Result<PackageInfo> {
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
    let checksum = get_commit_short_hash(repo, ref_path)?;

    Ok(PackageInfo {
        namespace: parts[2..boundary - 2].join("/"),
        slug: parts[boundary - 2].to_string(),
        version: parts[boundary - 1].to_string(),
        checksum,
    })
}

fn get_commit_short_hash(repo: &str, ref_path: &str) -> io::Result<String> {
    // use manifest hash (shared by all outputs of a build) - must match checkout.rs
    let ostree_repo = OstreeRepo::open(repo)?;

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
