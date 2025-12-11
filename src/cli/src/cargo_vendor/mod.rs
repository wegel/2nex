//! cargo_lock source type: vendors Rust dependencies by parsing Cargo.lock directly.
//!
//! this module parses a Cargo.lock file, downloads each crate from crates.io or git,
//! and creates a deterministic vendor tarball. Similar to Nix's importCargoLock.

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use toml::Value;

use crate::utils::{copy_dir_recursive, create_deterministic_tarball, fetch_url_or_file};

/// vendor Rust dependencies by parsing Cargo.lock directly.
/// returns path to the vendored tarball.
pub fn vendor_from_lock(
    cargo_lock_ref: &str,
    _cargo_toml_ref: Option<&str>,
    expected_sha256: &str,
    cache_dir: &str,
) -> io::Result<PathBuf> {
    // check cache first
    let vendor_cache = Path::new(cache_dir).join("cargo_vendor");
    let cache_path = vendor_cache.join(format!("sha256-{}.tar.gz", expected_sha256));

    if cache_path.exists() {
        // verify cached tarball
        let mut file = File::open(&cache_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        let calculated_hash = hex::encode(Sha256::digest(&contents));

        if calculated_hash == expected_sha256 {
            println!("Found cached vendor tarball: {}", cache_path.display());
            return Ok(cache_path);
        } else {
            println!(
                "Cached vendor tarball hash mismatch, re-vendoring (expected {}, got {})",
                expected_sha256, calculated_hash
            );
            fs::remove_file(&cache_path)?;
        }
    }

    // fetch Cargo.lock content
    println!("Fetching Cargo.lock from {}", cargo_lock_ref);
    let cargo_lock_content = fetch_url_or_file(cargo_lock_ref)?;

    // parse Cargo.lock
    let (registry_packages, git_packages) = parse_cargo_lock(&cargo_lock_content)?;

    let total = registry_packages.len() + git_packages.len();
    println!(
        "Found {} packages in Cargo.lock ({} crates.io, {} git)",
        total,
        registry_packages.len(),
        git_packages.len()
    );

    // create work directory for vendor
    let work_dir = tempfile::tempdir()?;
    let vendor_dir = work_dir.path().join("vendor");
    fs::create_dir_all(&vendor_dir)?;

    // download each crate
    let crate_cache = vendor_cache.join("crates");
    let git_cache = vendor_cache.join("git");
    fs::create_dir_all(&crate_cache)?;
    fs::create_dir_all(&git_cache)?;

    for pkg in &registry_packages {
        download_and_extract_registry_crate(pkg, &vendor_dir, &crate_cache)?;
    }

    for pkg in &git_packages {
        download_and_extract_git_crate(pkg, &vendor_dir, &git_cache)?;
    }

    // create deterministic tarball
    fs::create_dir_all(&vendor_cache)?;
    let tmp_tarball = cache_path.with_extension("tmp");
    create_deterministic_tarball(&vendor_dir, &tmp_tarball)?;

    // verify hash
    let mut file = File::open(&tmp_tarball)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    let calculated_hash = hex::encode(Sha256::digest(&contents));

    if calculated_hash != expected_sha256 {
        // keep the tarball for inspection but rename it
        let debug_path = vendor_cache.join(format!("sha256-{}.tar.gz", calculated_hash));
        fs::rename(&tmp_tarball, &debug_path)?;

        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Vendor tarball hash mismatch: expected {}, got {}.\nTarball saved to {} for inspection.",
                expected_sha256, calculated_hash, debug_path.display()
            ),
        ));
    }

    // move to final location
    fs::rename(&tmp_tarball, &cache_path)?;
    println!("Created vendor tarball: {}", cache_path.display());

    Ok(cache_path)
}

/// package info from crates.io registry
#[derive(Debug)]
struct RegistryPackage {
    name: String,
    version: String,
    checksum: String,
}

/// package info from git
#[derive(Debug)]
struct GitPackage {
    name: String,
    version: String,
    repo_url: String,
    commit_sha: String,
}

/// parse Cargo.lock and extract packages
fn parse_cargo_lock(content: &str) -> io::Result<(Vec<RegistryPackage>, Vec<GitPackage>)> {
    let lock: Value = content.parse().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse Cargo.lock: {}", e),
        )
    })?;

    let mut registry_packages = Vec::new();
    let mut git_packages = Vec::new();

    if let Some(pkg_array) = lock.get("package").and_then(|v| v.as_array()) {
        for pkg in pkg_array {
            let source = pkg.get("source").and_then(|v| v.as_str()).unwrap_or("");
            let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("");

            if name.is_empty() || version.is_empty() {
                continue;
            }

            if source.starts_with("registry+") {
                // crates.io package
                let checksum = pkg.get("checksum").and_then(|v| v.as_str()).unwrap_or("");
                if checksum.is_empty() {
                    continue;
                }
                registry_packages.push(RegistryPackage {
                    name: name.to_string(),
                    version: version.to_string(),
                    checksum: checksum.to_string(),
                });
            } else if source.starts_with("git+") {
                // git package: git+https://github.com/owner/repo?...#commitsha
                if let Some((repo_url, commit_sha)) = parse_git_source(source) {
                    git_packages.push(GitPackage {
                        name: name.to_string(),
                        version: version.to_string(),
                        repo_url,
                        commit_sha,
                    });
                }
            }
            // skip path dependencies (no source field or source is a path)
        }
    }

    Ok((registry_packages, git_packages))
}

/// parse git source string: git+https://github.com/owner/repo?rev=abc#fullsha
fn parse_git_source(source: &str) -> Option<(String, String)> {
    // format: git+<url>#<commit>
    let without_prefix = source.strip_prefix("git+")?;
    let parts: Vec<&str> = without_prefix.splitn(2, '#').collect();
    if parts.len() != 2 {
        return None;
    }

    // remove query params (?rev=..., ?branch=..., ?tag=...) from URL
    let url = parts[0].split('?').next()?;
    let commit = parts[1];

    Some((url.to_string(), commit.to_string()))
}

/// download and extract a crates.io package
fn download_and_extract_registry_crate(
    pkg: &RegistryPackage,
    vendor_dir: &Path,
    cache_dir: &Path,
) -> io::Result<()> {
    let crate_dir_name = format!("{}-{}", pkg.name, pkg.version);
    let dest_dir = vendor_dir.join(&crate_dir_name);

    if dest_dir.exists() {
        return Ok(());
    }

    // download crate tarball (cached)
    let crate_file = cache_dir.join(format!("{}.crate", crate_dir_name));
    if !crate_file.exists() {
        let url = format!(
            "https://static.crates.io/crates/{}/{}.crate",
            pkg.name, crate_dir_name
        );
        println!("  Downloading {}", crate_dir_name);

        let status = Command::new("curl")
            .args([
                "-L",
                "-f",
                "-s",
                "--output",
                crate_file.to_str().unwrap(),
                &url,
            ])
            .status()?;

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to download {}", url),
            ));
        }
    }

    // verify checksum
    let mut file = File::open(&crate_file)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    let calculated_hash = hex::encode(Sha256::digest(&contents));

    if calculated_hash != pkg.checksum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Checksum mismatch for {}: expected {}, got {}",
                crate_dir_name, pkg.checksum, calculated_hash
            ),
        ));
    }

    // extract to vendor directory
    let status = Command::new("tar")
        .args([
            "--no-same-owner",
            "-xf",
            crate_file.to_str().unwrap(),
            "-C",
            vendor_dir.to_str().unwrap(),
        ])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to extract {}", crate_dir_name),
        ));
    }

    // create .cargo-checksum.json
    let checksum_file = dest_dir.join(".cargo-checksum.json");
    let checksum_json = format!(r#"{{"files":{{}},"package":"{}"}}"#, pkg.checksum);
    fs::write(&checksum_file, checksum_json)?;

    Ok(())
}

/// download and extract a git package
fn download_and_extract_git_crate(
    pkg: &GitPackage,
    vendor_dir: &Path,
    cache_dir: &Path,
) -> io::Result<()> {
    let crate_dir_name = format!("{}-{}", pkg.name, pkg.version);
    let dest_dir = vendor_dir.join(&crate_dir_name);

    if dest_dir.exists() {
        return Ok(());
    }

    // cache by commit SHA
    let cached_dir = cache_dir.join(&pkg.commit_sha);

    if !cached_dir.exists() {
        println!(
            "  Downloading {} (git: {})",
            crate_dir_name,
            &pkg.commit_sha[..12]
        );

        // try GitHub-style archive URL first
        let archive_url = if pkg.repo_url.contains("github.com") {
            format!("{}/archive/{}.tar.gz", pkg.repo_url, pkg.commit_sha)
        } else if pkg.repo_url.contains("gitlab") {
            // GitLab format: /-/archive/<sha>/repo-<sha>.tar.gz
            let repo_name = pkg
                .repo_url
                .rsplit('/')
                .next()
                .unwrap_or("repo")
                .trim_end_matches(".git");
            format!(
                "{}/-/archive/{}/{}-{}.tar.gz",
                pkg.repo_url, pkg.commit_sha, repo_name, pkg.commit_sha
            )
        } else {
            // fall back to git clone for other hosts
            return download_git_clone(pkg, &cached_dir);
        };

        // download archive
        let tmp_archive = cache_dir.join(format!("{}.tar.gz", pkg.commit_sha));
        let status = Command::new("curl")
            .args([
                "-L",
                "-f",
                "-s",
                "--output",
                tmp_archive.to_str().unwrap(),
                &archive_url,
            ])
            .status()?;

        if !status.success() {
            // fall back to git clone
            return download_git_clone(pkg, &cached_dir);
        }

        // extract to temp location
        let tmp_extract = cache_dir.join(format!("tmp-{}", pkg.commit_sha));
        fs::create_dir_all(&tmp_extract)?;

        let status = Command::new("tar")
            .args([
                "--no-same-owner",
                "-xf",
                tmp_archive.to_str().unwrap(),
                "-C",
                tmp_extract.to_str().unwrap(),
            ])
            .status()?;

        if !status.success() {
            fs::remove_dir_all(&tmp_extract).ok();
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to extract git archive for {}", pkg.name),
            ));
        }

        // GitHub extracts to repo-sha/ directory, find and rename it
        let extracted_dirs: Vec<_> = fs::read_dir(&tmp_extract)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        if extracted_dirs.len() == 1 {
            fs::rename(extracted_dirs[0].path(), &cached_dir)?;
        } else {
            // multiple dirs or none - just use tmp_extract as cached_dir
            fs::rename(&tmp_extract, &cached_dir)?;
        }

        fs::remove_dir_all(&tmp_extract).ok();
        fs::remove_file(&tmp_archive).ok();
    }

    // copy from cache to vendor dir
    copy_dir_recursive(&cached_dir, &dest_dir)?;

    // create .cargo-checksum.json with null package (git deps have no crates.io checksum)
    let checksum_file = dest_dir.join(".cargo-checksum.json");
    fs::write(&checksum_file, r#"{"files":{},"package":null}"#)?;

    Ok(())
}

/// fall back to git clone for hosts that don't support archive downloads
fn download_git_clone(pkg: &GitPackage, dest_dir: &Path) -> io::Result<()> {
    let tmp_clone = dest_dir.with_extension("tmp");
    fs::create_dir_all(&tmp_clone)?;

    // shallow clone at specific commit
    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            &pkg.repo_url,
            tmp_clone.to_str().unwrap(),
        ])
        .status()?;

    if !status.success() {
        fs::remove_dir_all(&tmp_clone).ok();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to clone {}", pkg.repo_url),
        ));
    }

    // fetch specific commit (shallow clone might not have it)
    let status = Command::new("git")
        .args([
            "-C",
            tmp_clone.to_str().unwrap(),
            "fetch",
            "--depth",
            "1",
            "origin",
            &pkg.commit_sha,
        ])
        .status()?;

    if !status.success() {
        fs::remove_dir_all(&tmp_clone).ok();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Failed to fetch commit {} from {}",
                pkg.commit_sha, pkg.repo_url
            ),
        ));
    }

    // checkout the commit
    let status = Command::new("git")
        .args([
            "-C",
            tmp_clone.to_str().unwrap(),
            "checkout",
            &pkg.commit_sha,
        ])
        .status()?;

    if !status.success() {
        fs::remove_dir_all(&tmp_clone).ok();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to checkout commit {}", pkg.commit_sha),
        ));
    }

    // remove .git directory
    fs::remove_dir_all(tmp_clone.join(".git")).ok();

    fs::rename(&tmp_clone, dest_dir)?;
    Ok(())
}
