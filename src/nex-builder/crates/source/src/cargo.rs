//! Cargo.lock parsing and Cargo vendor archives.

use std::fs;
use std::io;
use std::path::Path;

use toml::Value;

use crate::{archive, cache, fetch, invalid};

#[derive(Clone, Debug, Eq, PartialEq)]
struct RegistryPackage {
    name: String,
    version: String,
    checksum: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GitPackage {
    name: String,
    version: String,
    url: String,
    commit: String,
}

pub(crate) fn vendor(
    lock_reference: &str,
    manifest: &Path,
    cache_root: &Path,
    output: &Path,
) -> io::Result<()> {
    let lock = fetch::text(lock_reference, manifest)?;
    let (registry, git) = parse_lock(&lock)?;
    let work = tempfile::tempdir()?;
    let vendor = work.path().join("vendor");
    fs::create_dir(&vendor)?;
    let cargo_cache = cache_root.join("cargo_vendor");
    for package in &registry {
        stage_registry(package, &cargo_cache.join("crates"), &vendor)?;
    }
    for package in &git {
        stage_git(package, &cargo_cache.join("git"), &vendor)?;
    }
    archive::create(&vendor, output)
}

fn parse_lock(content: &str) -> io::Result<(Vec<RegistryPackage>, Vec<GitPackage>)> {
    let lock: Value = content
        .parse()
        .map_err(|error| invalid(format!("invalid Cargo.lock: {error}")))?;
    let metadata = lock.get("metadata").and_then(Value::as_table);
    let mut registry = Vec::new();
    let mut git = Vec::new();
    let packages = lock
        .get("package")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("Cargo.lock has no package list"))?;
    for package in packages {
        let Some(name) = package.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(version) = package.get("version").and_then(Value::as_str) else {
            continue;
        };
        let source = package
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if source.starts_with("registry+") {
            let checksum = package
                .get("checksum")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| legacy_checksum(metadata, name, version, source))
                .ok_or_else(|| {
                    invalid(format!("registry crate {name} {version} has no checksum"))
                })?;
            registry.push(RegistryPackage {
                name: name.to_string(),
                version: version.to_string(),
                checksum,
            });
        } else if let Some((url, commit)) = parse_git_source(source) {
            git.push(GitPackage {
                name: name.to_string(),
                version: version.to_string(),
                url,
                commit,
            });
        }
    }
    Ok((registry, git))
}

fn legacy_checksum(
    metadata: Option<&toml::map::Map<String, Value>>,
    name: &str,
    version: &str,
    source: &str,
) -> Option<String> {
    metadata?
        .get(&format!("checksum {name} {version} ({source})"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn parse_git_source(source: &str) -> Option<(String, String)> {
    let source = source.strip_prefix("git+")?;
    let (url, commit) = source.split_once('#')?;
    Some((
        url.split('?').next()?.trim_end_matches(".git").to_string(),
        commit.to_string(),
    ))
}

fn stage_registry(package: &RegistryPackage, crate_cache: &Path, vendor: &Path) -> io::Result<()> {
    safe_component(&package.name)?;
    safe_component(&package.version)?;
    fs::create_dir_all(crate_cache)?;
    let directory = format!("{}-{}", package.name, package.version);
    let archive = crate_cache.join(format!("{directory}.crate"));
    cache::locked(&archive, || {
        if archive.is_file() && cache::verify(&archive, &package.checksum).is_ok() {
            return Ok(());
        }
        let work = tempfile::Builder::new()
            .prefix(".crate-")
            .tempdir_in(crate_cache)?;
        let temporary = work.path().join("download");
        let url = format!(
            "https://static.crates.io/crates/{}/{}.crate",
            package.name, directory
        );
        fetch::download(&url, &temporary)?;
        cache::verify(&temporary, &package.checksum)?;
        if archive.exists() {
            fs::remove_file(&archive)?;
        }
        fs::rename(temporary, &archive)
    })?;
    archive::extract_tar(&archive, vendor, false)?;
    let destination = vendor.join(&directory);
    if !destination.is_dir() {
        return Err(invalid(format!(
            "crate archive did not contain {directory}"
        )));
    }
    fs::write(
        destination.join(".cargo-checksum.json"),
        format!(r#"{{"files":{{}},"package":"{}"}}"#, package.checksum),
    )
}

fn stage_git(package: &GitPackage, git_cache: &Path, vendor: &Path) -> io::Result<()> {
    safe_component(&package.name)?;
    safe_component(&package.version)?;
    safe_component(&package.commit)?;
    fs::create_dir_all(git_cache)?;
    let cached = git_cache.join(&package.commit);
    cache::locked(&cached, || {
        if cached.is_dir() {
            return Ok(());
        }
        let work = tempfile::Builder::new()
            .prefix(".git-")
            .tempdir_in(git_cache)?;
        let checkout = work.path().join("checkout");
        if !download_git_archive(package, work.path(), &checkout)? {
            fetch::clone_git(&package.url, &package.commit, &checkout)?;
        }
        fs::rename(checkout, &cached)
    })?;
    let destination = vendor.join(format!("{}-{}", package.name, package.version));
    if !destination.exists() {
        archive::copy_tree(&cached, &destination)?;
    }
    fs::write(
        destination.join(".cargo-checksum.json"),
        r#"{"files":{},"package":null}"#,
    )
}

fn download_git_archive(package: &GitPackage, work: &Path, checkout: &Path) -> io::Result<bool> {
    let url = if package.url.contains("github.com") {
        format!("{}/archive/{}.tar.gz", package.url, package.commit)
    } else if package.url.contains("gitlab") {
        let name = package.url.rsplit('/').next().unwrap_or("repo");
        format!(
            "{}/-/archive/{}/{}-{}.tar.gz",
            package.url, package.commit, name, package.commit
        )
    } else {
        return Ok(false);
    };
    let downloaded = work.join("source.tar.gz");
    if fetch::download(&url, &downloaded).is_err() {
        return Ok(false);
    }
    let extracted = work.join("extracted");
    archive::extract_tar(&downloaded, &extracted, false)?;
    match archive::only_directory(&extracted)? {
        Some(directory) => fs::rename(directory, checkout)?,
        None => fs::rename(extracted, checkout)?,
    }
    Ok(true)
}

fn safe_component(value: &str) -> io::Result<()> {
    if crate::safe_token(value, b"-_.+") {
        Ok(())
    } else {
        Err(invalid(format!("unsafe Cargo package component {value:?}")))
    }
}

#[cfg(test)]
#[path = "cargo_tests.rs"]
mod tests;
