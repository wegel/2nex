//! zig_zon source type: vendors Zig dependencies by parsing build.zig.zon.
//!
//! this module parses a build.zig.zon file, downloads each dependency (URL or git),
//! computes the Zig multihash for each, and creates a tarball with the structure
//! p/<hash>/... that can be extracted to ZIG_GLOBAL_CACHE_DIR for offline builds.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::{copy_dir_recursive, create_deterministic_tarball, fetch_url_or_file};

/// vendor Zig dependencies by parsing build.zig.zon.
/// returns path to the vendored tarball.
pub fn vendor_from_zon(
    zig_zon_ref: &str,
    expected_sha256: &str,
    cache_dir: &str,
) -> io::Result<PathBuf> {
    // check cache first
    let vendor_cache = Path::new(cache_dir).join("zig_vendor");
    let cache_path = vendor_cache.join(format!("sha256-{}.tar.gz", expected_sha256));

    if cache_path.exists() {
        // verify cached tarball
        let mut file = File::open(&cache_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        let calculated_hash = hex::encode(Sha256::digest(&contents));

        if calculated_hash == expected_sha256 {
            println!("Found cached Zig vendor tarball: {}", cache_path.display());
            return Ok(cache_path);
        } else {
            println!(
                "Cached Zig vendor tarball hash mismatch, re-vendoring (expected {}, got {})",
                expected_sha256, calculated_hash
            );
            fs::remove_file(&cache_path)?;
        }
    }

    // fetch build.zig.zon content
    println!("Fetching build.zig.zon from {}", zig_zon_ref);
    let zon_content = fetch_url_or_file(zig_zon_ref)?;

    // parse build.zig.zon
    let deps = parse_zig_zon(&zon_content)?;

    println!("Found {} dependencies in build.zig.zon", deps.len());

    // create work directory for vendor
    let work_dir = tempfile::tempdir()?;
    let vendor_dir = work_dir.path().join("p");
    fs::create_dir_all(&vendor_dir)?;

    // download cache
    let download_cache = vendor_cache.join("downloads");
    fs::create_dir_all(&download_cache)?;

    // process each dependency (and their transitive deps)
    let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut pending = deps;

    while !pending.is_empty() {
        let mut new_deps = Vec::new();

        for dep in &pending {
            if processed.contains(&dep.hash) {
                continue;
            }
            processed.insert(dep.hash.clone());

            let transitive = download_and_prepare_dep(dep, &vendor_dir, &download_cache)?;
            new_deps.extend(transitive);
        }

        pending = new_deps;
    }

    // create deterministic tarball from the p/ directory
    // the tarball will contain paths like p/<hash>/...
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
                "Zig vendor tarball hash mismatch: expected {}, got {}.\nTarball saved to {} for inspection.",
                expected_sha256, calculated_hash, debug_path.display()
            ),
        ));
    }

    // move to final location
    fs::rename(&tmp_tarball, &cache_path)?;
    println!("Created Zig vendor tarball: {}", cache_path.display());

    Ok(cache_path)
}

/// dependency info from build.zig.zon
#[derive(Debug, Clone)]
pub struct ZigDep {
    pub name: String,
    pub url: String,
    pub hash: String,
    /// git revision if this is a git URL
    pub rev: Option<String>,
}

/// parse build.zig.zon and extract dependencies.
/// ZON is a subset of Zig syntax, we parse it manually.
fn parse_zig_zon(content: &str) -> io::Result<Vec<ZigDep>> {
    let mut deps = Vec::new();

    // find .dependencies = .{ ... }
    let deps_start = match content.find(".dependencies") {
        Some(pos) => pos,
        None => return Ok(deps), // no dependencies
    };

    // find the opening brace after .dependencies
    let after_deps = &content[deps_start..];
    let brace_start = match after_deps.find(".{") {
        Some(pos) => deps_start + pos,
        None => return Ok(deps),
    };

    // find matching closing brace
    let deps_block = extract_brace_block(&content[brace_start..])?;

    // parse each dependency: .name = .{ .url = "...", .hash = "..." }
    let mut pos = 0;
    while pos < deps_block.len() {
        // find next dependency name starting with .
        let remaining = &deps_block[pos..];

        // skip whitespace and commas
        let trimmed = remaining.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        if trimmed.is_empty() || trimmed.starts_with('}') {
            break;
        }
        pos += remaining.len() - trimmed.len();

        // find .name = .{
        if !trimmed.starts_with('.') {
            pos += 1;
            continue;
        }

        // extract dependency name
        let name_end = trimmed[1..]
            .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .map(|i| i + 1)
            .unwrap_or(trimmed.len());
        let name = &trimmed[1..name_end];

        // find .{ after =
        let after_name = &trimmed[name_end..];
        let eq_pos = match after_name.find('=') {
            Some(p) => p,
            None => {
                pos += name_end;
                continue;
            }
        };

        let after_eq = after_name[eq_pos + 1..].trim_start();
        if !after_eq.starts_with(".{") {
            pos += name_end + eq_pos + 1;
            continue;
        }

        // extract the dependency block
        let dep_block = match extract_brace_block(after_eq) {
            Ok(b) => b,
            Err(_) => {
                pos += name_end + eq_pos + 1;
                continue;
            }
        };

        // parse url and hash from dep_block
        let url = extract_string_field(&dep_block, ".url");
        let hash = extract_string_field(&dep_block, ".hash");

        if let (Some(url_str), Some(hash_str)) = (url, hash) {
            // check if it's a git URL (git+https://...#revision)
            let (final_url, rev) = if url_str.starts_with("git+") {
                parse_git_url(&url_str)
            } else {
                (url_str, None)
            };

            deps.push(ZigDep {
                name: name.to_string(),
                url: final_url,
                hash: hash_str,
                rev,
            });
        }

        pos += name_end + eq_pos + 1 + dep_block.len() + 4; // .{ and }
    }

    Ok(deps)
}

/// extract a brace-delimited block, handling nested braces
fn extract_brace_block(s: &str) -> io::Result<String> {
    let s = s.trim_start();
    if !s.starts_with(".{") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Expected .{ at start of block",
        ));
    }

    let mut depth = 0;
    let mut end = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, c) in s.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match c {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unbalanced braces in ZON",
        ));
    }

    // return content between .{ and }
    Ok(s[2..end].to_string())
}

/// extract a string value for a given field name, skipping comments
fn extract_string_field(block: &str, field: &str) -> Option<String> {
    // first, strip comments from the block
    let clean_block = strip_zon_comments(block);

    let field_pos = clean_block.find(field)?;
    let after_field = &clean_block[field_pos + field.len()..];

    // find = and then the string
    let eq_pos = after_field.find('=')?;
    let after_eq = after_field[eq_pos + 1..].trim_start();

    // string starts with "
    if !after_eq.starts_with('"') {
        return None;
    }

    // find closing quote (handling escapes)
    let mut end = 1;
    let mut escape_next = false;
    for (i, c) in after_eq[1..].char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        match c {
            '\\' => escape_next = true,
            '"' => {
                end = i + 1;
                break;
            }
            _ => {}
        }
    }

    Some(after_eq[1..end].to_string())
}

/// strip single-line comments (//) from ZON content
fn strip_zon_comments(s: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut escape_next = false;
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if escape_next {
            escape_next = false;
            result.push(c);
            continue;
        }

        match c {
            '\\' if in_string => {
                escape_next = true;
                result.push(c);
            }
            '"' => {
                in_string = !in_string;
                result.push(c);
            }
            '/' if !in_string => {
                if chars.peek() == Some(&'/') {
                    // skip until end of line
                    chars.next(); // consume second /
                    while let Some(&next) = chars.peek() {
                        if next == '\n' {
                            break;
                        }
                        chars.next();
                    }
                } else {
                    result.push(c);
                }
            }
            _ => result.push(c),
        }
    }

    result
}

/// parse git URL: git+https://github.com/foo/bar.git?ref=HEAD#commit
fn parse_git_url(url: &str) -> (String, Option<String>) {
    let without_prefix = url.strip_prefix("git+").unwrap_or(url);

    // split on # to get revision
    let (base, rev) = if let Some(hash_pos) = without_prefix.rfind('#') {
        let rev = &without_prefix[hash_pos + 1..];
        let base = &without_prefix[..hash_pos];
        (base.to_string(), Some(rev.to_string()))
    } else {
        (without_prefix.to_string(), None)
    };

    // remove query params (?ref=HEAD, etc)
    let clean_base = base.split('?').next().unwrap_or(&base).to_string();

    (clean_base, rev)
}

/// check if a hash is in the new named format (contains -)
fn is_named_hash(hash: &str) -> bool {
    // named hashes have format: name-semver-sizedhash
    // old hashes start with 1220 (hex for sha256 multihash)
    hash.contains('-') && !hash.starts_with("1220")
}

/// download a dependency and prepare it in the vendor directory.
/// returns any transitive dependencies found.
fn download_and_prepare_dep(
    dep: &ZigDep,
    vendor_dir: &Path,
    cache_dir: &Path,
) -> io::Result<Vec<ZigDep>> {
    // destination is vendor_dir/<hash>
    let dest_dir = vendor_dir.join(&dep.hash);

    if dest_dir.exists() {
        return Ok(Vec::new());
    }

    println!(
        "  Downloading {} ({})",
        dep.name,
        &dep.hash[..30.min(dep.hash.len())]
    );

    // download to cache (use sanitized name for cache dir to avoid path issues)
    let cache_name = dep.hash.replace('/', "_");
    let cached_dir = cache_dir.join(&cache_name);

    if !cached_dir.exists() {
        if let Some(ref rev) = dep.rev {
            // git dependency
            download_git_dep(&dep.url, rev, &cached_dir)?;
        } else {
            // URL dependency (tarball)
            download_url_dep(&dep.url, &cached_dir)?;
        }
    }

    // for old 1220... format, verify the hash
    // for new named format, we trust the upstream hash (we can't compute it ourselves
    // because it includes the package fingerprint which we don't have access to)
    if !is_named_hash(&dep.hash) {
        let computed_hash = compute_zig_hash(&cached_dir)?;
        if computed_hash != dep.hash {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Zig hash mismatch for {}: expected {}, computed {}",
                    dep.name, dep.hash, computed_hash
                ),
            ));
        }
    }

    // copy to vendor directory
    copy_dir_recursive(&cached_dir, &dest_dir)?;

    // check for transitive dependencies - both in root and any subdirectory build.zig.zon
    let mut transitive_deps = Vec::new();

    // scan the entire downloaded package for build.zig.zon files
    for entry in walkdir::WalkDir::new(&cached_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "build.zig.zon" {
            if let Ok(zon_content) = fs::read_to_string(entry.path()) {
                if let Ok(deps) = parse_zig_zon(&zon_content) {
                    for dep in deps {
                        // avoid duplicates
                        if !transitive_deps.iter().any(|d: &ZigDep| d.hash == dep.hash) {
                            transitive_deps.push(dep);
                        }
                    }
                }
            }
        }
    }

    Ok(transitive_deps)
}

/// download a git dependency
fn download_git_dep(url: &str, rev: &str, dest: &Path) -> io::Result<()> {
    let tmp_dir = dest.with_extension("tmp");
    fs::create_dir_all(&tmp_dir)?;

    // try archive download first (faster for GitHub/GitLab)
    if url.contains("github.com") {
        let archive_url = format!("{}/archive/{}.tar.gz", url.trim_end_matches(".git"), rev);
        if try_download_archive(&archive_url, &tmp_dir)? {
            fs::rename(&tmp_dir, dest)?;
            return Ok(());
        }
    }

    // fall back to git clone
    let status = Command::new("git")
        .args(["clone", "--depth", "1", url, tmp_dir.to_str().unwrap()])
        .status()?;

    if !status.success() {
        fs::remove_dir_all(&tmp_dir).ok();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to clone {}", url),
        ));
    }

    // fetch specific commit
    let status = Command::new("git")
        .args([
            "-C",
            tmp_dir.to_str().unwrap(),
            "fetch",
            "--depth",
            "1",
            "origin",
            rev,
        ])
        .status()?;

    if !status.success() {
        // might already have it, try checkout anyway
    }

    let status = Command::new("git")
        .args(["-C", tmp_dir.to_str().unwrap(), "checkout", rev])
        .status()?;

    if !status.success() {
        fs::remove_dir_all(&tmp_dir).ok();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to checkout {} from {}", rev, url),
        ));
    }

    // remove .git directory
    fs::remove_dir_all(tmp_dir.join(".git")).ok();

    fs::rename(&tmp_dir, dest)?;
    Ok(())
}

/// download a URL dependency (tarball)
fn download_url_dep(url: &str, dest: &Path) -> io::Result<()> {
    let tmp_archive = dest.with_extension("tar.gz");

    let status = Command::new("curl")
        .args([
            "-L",
            "-f",
            "-s",
            "--output",
            tmp_archive.to_str().unwrap(),
            url,
        ])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to download {}", url),
        ));
    }

    // extract
    let tmp_extract = dest.with_extension("extract");
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

    fs::remove_file(&tmp_archive).ok();

    if !status.success() {
        fs::remove_dir_all(&tmp_extract).ok();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to extract {}", url),
        ));
    }

    // GitHub-style archives extract to repo-sha/, move contents up
    let entries: Vec<_> = fs::read_dir(&tmp_extract)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    if entries.len() == 1 {
        fs::rename(entries[0].path(), dest)?;
        fs::remove_dir_all(&tmp_extract).ok();
    } else {
        fs::rename(&tmp_extract, dest)?;
    }

    Ok(())
}

/// try to download and extract an archive, return true on success
fn try_download_archive(url: &str, dest: &Path) -> io::Result<bool> {
    let tmp_archive = dest.with_extension("tar.gz");

    let status = Command::new("curl")
        .args([
            "-L",
            "-f",
            "-s",
            "--output",
            tmp_archive.to_str().unwrap(),
            url,
        ])
        .status()?;

    if !status.success() {
        return Ok(false);
    }

    let status = Command::new("tar")
        .args([
            "--no-same-owner",
            "-xf",
            tmp_archive.to_str().unwrap(),
            "-C",
            dest.to_str().unwrap(),
            "--strip-components=1",
        ])
        .status()?;

    fs::remove_file(&tmp_archive).ok();

    Ok(status.success())
}

/// compute Zig's multihash for a directory.
/// format: "1220" + sha256(sorted file hashes)
/// each file hash = sha256(normalized_path + \0\0 + content)
pub fn compute_zig_hash(dir: &Path) -> io::Result<String> {
    let mut file_hashes: BTreeMap<String, [u8; 32]> = BTreeMap::new();

    collect_file_hashes(dir, dir, &mut file_hashes)?;

    // combine all hashes in sorted order
    let mut combined_hasher = Sha256::new();
    for (_path, hash) in &file_hashes {
        combined_hasher.update(hash);
    }

    let final_hash = combined_hasher.finalize();

    // zig multihash: 0x12 (sha256) + 0x20 (32 bytes) + hash
    Ok(format!("1220{}", hex::encode(final_hash)))
}

/// recursively collect file hashes
fn collect_file_hashes(
    base: &Path,
    dir: &Path,
    hashes: &mut BTreeMap<String, [u8; 32]>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        // get relative path from base, normalized with forward slashes
        let rel_path = path
            .strip_prefix(base)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let normalized = rel_path
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");

        if file_type.is_dir() {
            collect_file_hashes(base, &path, hashes)?;
        } else if file_type.is_symlink() {
            // for symlinks, hash the target path
            let target = fs::read_link(&path)?;
            let target_str = target.to_string_lossy();

            let mut hasher = Sha256::new();
            hasher.update(normalized.as_bytes());
            hasher.update(b"\0\0");
            hasher.update(target_str.as_bytes());

            let hash: [u8; 32] = hasher.finalize().into();
            hashes.insert(normalized, hash);
        } else if file_type.is_file() {
            // for regular files, hash path + content
            let content = fs::read(&path)?;

            let mut hasher = Sha256::new();
            hasher.update(normalized.as_bytes());
            hasher.update(b"\0\0");
            hasher.update(&content);

            let hash: [u8; 32] = hasher.finalize().into();
            hashes.insert(normalized, hash);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_url() {
        let (url, rev) = parse_git_url("git+https://github.com/foo/bar.git?ref=HEAD#abc123");
        assert_eq!(url, "https://github.com/foo/bar.git");
        assert_eq!(rev, Some("abc123".to_string()));

        let (url, rev) = parse_git_url("git+https://github.com/foo/bar#def456");
        assert_eq!(url, "https://github.com/foo/bar");
        assert_eq!(rev, Some("def456".to_string()));
    }

    #[test]
    fn test_extract_string_field() {
        let block = r#".url = "https://example.com/pkg.tar.gz", .hash = "1220abc""#;
        assert_eq!(
            extract_string_field(block, ".url"),
            Some("https://example.com/pkg.tar.gz".to_string())
        );
        assert_eq!(
            extract_string_field(block, ".hash"),
            Some("1220abc".to_string())
        );
    }
}
