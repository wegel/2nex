//! Switch the active version of a package
//!
//! Updates symlinks in /usr/bin to point to a different installed version.

use clap::Args;
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::Path;

use super::stage::is_staged;
use super::state::InstalledState;

const NEX_PKG_DIR: &str = "/nex/pkg";
const USR_BIN_DIR: &str = "/usr/bin";

#[derive(Args)]
pub struct SwitchArgs {
    /// Package to switch (e.g., "bash" or "cli/shells/bash")
    pub package: String,

    /// Version to switch to
    pub version: String,

    /// Skip staging check
    #[clap(long, hide = true)]
    pub no_stage_check: bool,
}

pub fn run(args: &SwitchArgs) -> io::Result<()> {
    // check staging mode
    if !args.no_stage_check && !is_staged() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Not in staging mode. Use 'nex stage' first.",
        ));
    }

    // load state
    let mut state = InstalledState::load()?;

    // find the package
    let (namespace, slug) = parse_package_query(&args.package, &state)?;
    let pkg_key = format!("{}/{}", namespace, slug);

    let pkg_state = state.get_package(&namespace, &slug).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Package {} not installed", pkg_key),
        )
    })?;

    // find the version to switch to
    let target_version = find_version(&args.version, pkg_state)?;
    let (version, checksum) = parse_version_key(&target_version)?;

    // check if already current
    if pkg_state.current.as_deref() == Some(&target_version) {
        println!(
            "{} {} is already the current version",
            pkg_key, target_version
        );
        return Ok(());
    }

    // get the version info
    let version_info = pkg_state.versions.get(&target_version).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Version {} not found", target_version),
        )
    })?;

    // build the package directory path
    let pkg_dir = format!(
        "{}/{}/{}/{}/{}",
        NEX_PKG_DIR, namespace, slug, version, checksum
    );

    if !Path::new(&pkg_dir).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Package directory not found: {}", pkg_dir),
        ));
    }

    println!(
        "Switching {}/{} to version {}",
        namespace, slug, target_version
    );

    // update symlinks for each binary
    for binary in &version_info.provides {
        let src_path = format!("{}/usr/bin/{}", pkg_dir, binary);
        let dst = format!("{}/{}", USR_BIN_DIR, binary);
        // relative symlink: from /usr/bin/, ../../ reaches /, then nex/pkg/...
        let relative_target = format!(
            "../../nex/pkg/{}/{}/{}/{}/usr/bin/{}",
            namespace, slug, version, checksum, binary
        );

        if Path::new(&src_path).exists() {
            // remove existing symlink
            if Path::new(&dst).exists() || Path::new(&dst).is_symlink() {
                fs::remove_file(&dst)?;
            }
            symlink(&relative_target, &dst)?;
            println!("  {} -> {}", binary, relative_target);
        } else {
            eprintln!("  Warning: {} not found in {}", pkg_dir, binary);
        }
    }

    // update state
    state.switch_current(&namespace, &slug, &version, &checksum)?;
    state.save()?;

    println!("Switched to {} {}", pkg_key, target_version);

    Ok(())
}

/// Parse package query to find namespace and slug
fn parse_package_query(query: &str, state: &InstalledState) -> io::Result<(String, String)> {
    // check if query contains namespace
    if query.contains('/') {
        // could be full path like "cli/shells/bash" or partial like "shells/bash"
        let parts: Vec<&str> = query.rsplitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok((parts[1].to_string(), parts[0].to_string()));
        }
    }

    // search for matching package by slug
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
    // exact match
    if pkg_state.versions.contains_key(query) {
        return Ok(query.to_string());
    }

    // match by version prefix (e.g., "5.2.21" matches "5.2.21/a19536f4")
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
