//! Remove an installed package version
//!
//! Removes package files and updates symlinks.

use clap::Args;
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::Path;

use super::stage::is_staged;
use super::state::InstalledState;
use crate::repo::detect_context;

#[derive(Args)]
pub struct RemoveArgs {
    /// Package to remove (e.g., "bash" or "cli/shells/bash")
    pub package: String,

    /// Specific version to remove (default: current version)
    #[clap(long)]
    pub version: Option<String>,

    /// Remove all versions of the package
    #[clap(long)]
    pub all: bool,

    /// Skip staging check
    #[clap(long, hide = true)]
    pub no_stage_check: bool,

    /// Remove from system-wide installation (requires root)
    #[clap(long)]
    pub system: bool,
}

pub fn run(args: &RemoveArgs) -> io::Result<()> {
    // detect context
    let ctx = detect_context(args.system)?;

    // check staging mode for system installs only
    if ctx.is_system && ctx.needs_staging && !args.no_stage_check && !is_staged() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Not in staging mode. Use 'nex stage' first.",
        ));
    }

    // load state (context-aware)
    let mut state = InstalledState::load_for_context(&ctx)?;

    // determine paths based on context
    let nex_pkg_dir = ctx.pkg_path.to_string_lossy().to_string();
    let bin_dir = if ctx.is_system {
        "/usr/bin".to_string()
    } else {
        format!("{}/default/bin", ctx.env_path.display())
    };

    // find the package
    let (namespace, slug) = parse_package_query(&args.package, &state)?;
    let pkg_key = format!("{}/{}", namespace, slug);

    let pkg_state = state
        .get_package(&namespace, &slug)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Package {} not installed", pkg_key),
            )
        })?
        .clone();

    if args.all {
        // remove all versions
        println!("Removing all versions of {}...", pkg_key);

        // remove symlinks first (from current version)
        if let Some(current) = &pkg_state.current {
            if let Some(info) = pkg_state.versions.get(current) {
                remove_symlinks(&info.provides, &bin_dir)?;
            }
        }

        // remove all version directories
        for version_key in pkg_state.versions.keys() {
            let (version, checksum) = parse_version_key(version_key)?;
            let pkg_dir = format!(
                "{}/{}/{}/{}/{}",
                nex_pkg_dir, namespace, slug, version, checksum
            );
            remove_package_dir(&pkg_dir)?;
        }

        // update state - remove entire package
        let key = format!("{}/{}", namespace, slug);
        state.packages.remove(&key);

        println!(
            "Removed {} ({} version(s))",
            pkg_key,
            pkg_state.versions.len()
        );
    } else {
        // remove specific version
        let target_version = if let Some(v) = &args.version {
            find_version(v, &pkg_state)?
        } else {
            // default to current version
            pkg_state.current.clone().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "No current version for {}. Use --version to specify.",
                        pkg_key
                    ),
                )
            })?
        };

        let (version, checksum) = parse_version_key(&target_version)?;
        let is_current = pkg_state.current.as_deref() == Some(&target_version);

        println!("Removing {}/{} {}...", namespace, slug, target_version);

        // get version info before removing
        let version_info = pkg_state
            .versions
            .get(&target_version)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Version {} not found", target_version),
                )
            })?
            .clone();

        // remove from state first (this handles switching current if needed)
        let removed_info = state.record_remove(&namespace, &slug, &version, &checksum);

        if is_current {
            // need to update symlinks
            let new_current = state.get_current_version(&namespace, &slug);

            if let Some(new_ver) = new_current {
                // switch symlinks to the new current version
                println!("  Switching symlinks to {}", new_ver);
                if let Some(pkg) = state.get_package(&namespace, &slug) {
                    if let Some(new_info) = pkg.versions.get(new_ver) {
                        let (new_version, new_checksum) = parse_version_key(new_ver)?;
                        let new_pkg_dir = format!(
                            "{}/{}/{}/{}/{}",
                            nex_pkg_dir, namespace, slug, new_version, new_checksum
                        );
                        update_symlinks(&new_info.provides, &new_pkg_dir, &bin_dir)?;
                    }
                }
            } else {
                // no versions left, remove symlinks
                println!("  Removing symlinks (no versions remaining)");
                remove_symlinks(&version_info.provides, &bin_dir)?;
            }
        }

        // remove the package directory
        let pkg_dir = format!(
            "{}/{}/{}/{}/{}",
            nex_pkg_dir, namespace, slug, version, checksum
        );
        remove_package_dir(&pkg_dir)?;

        // clean up empty parent directories
        cleanup_empty_dirs(&format!(
            "{}/{}/{}/{}",
            nex_pkg_dir, namespace, slug, version
        ))?;
        cleanup_empty_dirs(&format!("{}/{}/{}", nex_pkg_dir, namespace, slug))?;

        if removed_info.is_some() {
            println!("Removed {}/{} {}", namespace, slug, target_version);
        }
    }

    // save state (context-aware)
    state.save_for_context(&ctx)?;

    Ok(())
}

/// Parse package query to find namespace and slug
fn parse_package_query(query: &str, state: &InstalledState) -> io::Result<(String, String)> {
    if query.contains('/') {
        let parts: Vec<&str> = query.rsplitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok((parts[1].to_string(), parts[0].to_string()));
        }
    }

    let matches: Vec<_> = state
        .packages
        .keys()
        .filter(|k| k.ends_with(&format!("/{}", query)) || k == &query)
        .collect();

    match matches.len() {
        0 => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("No installed package matching '{}'", query),
        )),
        1 => {
            let key = matches[0];
            let parts: Vec<&str> = key.rsplitn(2, '/').collect();
            Ok((parts[1].to_string(), parts[0].to_string()))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Ambiguous package '{}'. Matches: {}",
                query,
                matches
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}

/// Find version key from partial version string
fn find_version(query: &str, pkg_state: &super::state::PackageState) -> io::Result<String> {
    if pkg_state.versions.contains_key(query) {
        return Ok(query.to_string());
    }

    let matches: Vec<_> = pkg_state
        .versions
        .keys()
        .filter(|k| k.starts_with(&format!("{}/", query)) || k.starts_with(query))
        .collect();

    match matches.len() {
        0 => {
            let available: Vec<_> = pkg_state.versions.keys().map(|s| s.as_str()).collect();
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Version '{}' not found. Available: {}",
                    query,
                    available.join(", ")
                ),
            ))
        }
        1 => Ok(matches[0].clone()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Ambiguous version '{}'. Matches: {}",
                query,
                matches
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}

/// Parse version key "version/checksum" into parts
fn parse_version_key(key: &str) -> io::Result<(String, String)> {
    let parts: Vec<&str> = key.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid version key: {}", key),
        ));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Remove symlinks for the given binaries
fn remove_symlinks(binaries: &[String], bin_dir: &str) -> io::Result<()> {
    for binary in binaries {
        let dst = format!("{}/{}", bin_dir, binary);
        if Path::new(&dst).is_symlink() {
            fs::remove_file(&dst)?;
            println!("  Removed symlink {}", binary);
        }
    }
    Ok(())
}

/// Update symlinks to point to a new package directory
fn update_symlinks(binaries: &[String], pkg_dir: &str, bin_dir: &str) -> io::Result<()> {
    for binary in binaries {
        let src = format!("{}/usr/bin/{}", pkg_dir, binary);
        let dst = format!("{}/{}", bin_dir, binary);

        if Path::new(&src).exists() {
            if Path::new(&dst).exists() || Path::new(&dst).is_symlink() {
                fs::remove_file(&dst)?;
            }
            symlink(&src, &dst)?;
            println!("  {} -> {}", binary, src);
        }
    }
    Ok(())
}

/// Remove a package directory
fn remove_package_dir(pkg_dir: &str) -> io::Result<()> {
    let path = Path::new(pkg_dir);
    if path.exists() {
        fs::remove_dir_all(path)?;
        println!("  Removed {}", pkg_dir);
    }
    Ok(())
}

/// Clean up empty directories
fn cleanup_empty_dirs(dir: &str) -> io::Result<()> {
    let path = Path::new(dir);
    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            if entries.count() == 0 {
                fs::remove_dir(path).ok();
            }
        }
    }
    Ok(())
}
