//! build.zig.zon parsing and Zig package-cache archives.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::io;
use std::path::Path;

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::{archive, cache, fetch, invalid};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Dependency {
    name: String,
    url: String,
    hash: String,
    revision: Option<String>,
}

pub(crate) fn vendor(
    zon_reference: &str,
    manifest: &Path,
    cache_root: &Path,
    output: &Path,
) -> io::Result<()> {
    let zon = fetch::text(zon_reference, manifest)?;
    let work = tempfile::tempdir()?;
    let packages = work.path().join("p");
    fs::create_dir(&packages)?;
    let downloads = cache_root.join("zig_vendor/downloads");
    fs::create_dir_all(&downloads)?;
    let mut pending = VecDeque::from(parse_zon(&zon)?);
    let mut seen = BTreeSet::new();
    while let Some(dependency) = pending.pop_front() {
        if !seen.insert(dependency.hash.clone()) {
            continue;
        }
        pending.extend(stage(&dependency, &downloads, &packages)?);
    }
    archive::create(&packages, output)
}

fn parse_zon(content: &str) -> io::Result<Vec<Dependency>> {
    let content = strip_comments(content);
    let Some(start) = content.find(".dependencies") else {
        return Ok(Vec::new());
    };
    let assignment = &content[start + ".dependencies".len()..];
    let block_start = assignment
        .find(".{")
        .ok_or_else(|| invalid("dependencies field is not a struct"))?;
    let (block, _) = brace_block(&assignment[block_start..])?;
    parse_dependency_block(block)
}

fn parse_dependency_block(mut block: &str) -> io::Result<Vec<Dependency>> {
    let mut dependencies = Vec::new();
    while let Some(dot) = block.find('.') {
        block = &block[dot + 1..];
        let name_end = block
            .find(|character: char| {
                !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
            })
            .unwrap_or(block.len());
        let name = &block[..name_end];
        let after_name = &block[name_end..];
        let Some(equals) = after_name.find('=') else {
            break;
        };
        let value = after_name[equals + 1..].trim_start();
        if !value.starts_with(".{") {
            block = value;
            continue;
        }
        let (fields, consumed) = brace_block(value)?;
        if let (Some(url), Some(hash)) =
            (string_field(fields, ".url"), string_field(fields, ".hash"))
        {
            let (url, revision) = git_url(&url);
            dependencies.push(Dependency {
                name: name.to_string(),
                url,
                hash,
                revision,
            });
        }
        block = &value[consumed..];
    }
    Ok(dependencies)
}

fn brace_block(value: &str) -> io::Result<(&str, usize)> {
    if !value.starts_with(".{") {
        return Err(invalid("ZON block does not start with .{"));
    }
    let mut depth = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
        } else if quoted && character == '\\' {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if !quoted && character == '{' {
            depth += 1;
        } else if !quoted && character == '}' {
            depth -= 1;
            if depth == 0 {
                return Ok((&value[2..index], index + character.len_utf8()));
            }
        }
    }
    Err(invalid("unbalanced braces in build.zig.zon"))
}

fn string_field(block: &str, name: &str) -> Option<String> {
    let after_name = &block[block.find(name)? + name.len()..];
    let value = after_name[after_name.find('=')? + 1..].trim_start();
    let quoted = value.strip_prefix('"')?;
    let mut escaped = false;
    for (index, character) in quoted.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(quoted[..index].to_string());
        }
    }
    None
}

fn strip_comments(content: &str) -> String {
    let mut output = String::with_capacity(content.len());
    let mut characters = content.chars().peekable();
    let mut quoted = false;
    let mut escaped = false;
    while let Some(character) = characters.next() {
        if escaped {
            escaped = false;
            output.push(character);
        } else if quoted && character == '\\' {
            escaped = true;
            output.push(character);
        } else if character == '"' {
            quoted = !quoted;
            output.push(character);
        } else if !quoted && character == '/' && characters.peek() == Some(&'/') {
            characters.next();
            while characters.next_if(|next| *next != '\n').is_some() {}
            output.push('\n');
        } else {
            output.push(character);
        }
    }
    output
}

fn git_url(value: &str) -> (String, Option<String>) {
    let Some(value) = value.strip_prefix("git+") else {
        return (value.to_string(), None);
    };
    let (url, revision) = value
        .rsplit_once('#')
        .map_or((value, None), |(url, revision)| {
            (url, Some(revision.to_string()))
        });
    (
        url.split('?')
            .next()
            .unwrap_or(url)
            .trim_end_matches(".git")
            .to_string(),
        revision,
    )
}

fn stage(
    dependency: &Dependency,
    downloads: &Path,
    packages: &Path,
) -> io::Result<Vec<Dependency>> {
    safe_hash(&dependency.hash)?;
    let cached = downloads.join(dependency.hash.replace('/', "_"));
    cache::locked(&cached, || {
        if cached.is_dir() {
            return Ok(());
        }
        let work = tempfile::Builder::new()
            .prefix(".zig-")
            .tempdir_in(downloads)?;
        let checkout = work.path().join("checkout");
        match &dependency.revision {
            Some(revision) => download_git(&dependency.url, revision, work.path(), &checkout)?,
            None => download_archive(&dependency.url, work.path(), &checkout)?,
        }
        fs::rename(checkout, &cached)
    })?;
    if !dependency.hash.contains('-') || dependency.hash.starts_with("1220") {
        let actual = zig_hash(&cached)?;
        if actual != dependency.hash {
            return Err(invalid(format!(
                "Zig hash mismatch for {}: expected {}, got {actual}",
                dependency.name, dependency.hash
            )));
        }
    }
    archive::copy_tree(&cached, &packages.join(&dependency.hash))?;
    transitive_dependencies(&cached)
}

fn download_git(url: &str, revision: &str, work: &Path, checkout: &Path) -> io::Result<()> {
    if url.contains("github.com") {
        let archive_url = format!("{url}/archive/{revision}.tar.gz");
        let downloaded = work.join("source.tar.gz");
        if fetch::download(&archive_url, &downloaded).is_ok()
            && archive::extract_tar(&downloaded, checkout, true).is_ok()
        {
            return Ok(());
        }
        if checkout.exists() {
            fs::remove_dir_all(checkout)?;
        }
    }
    fetch::clone_git(url, revision, checkout)
}

fn download_archive(url: &str, work: &Path, checkout: &Path) -> io::Result<()> {
    let downloaded = work.join("source.tar");
    fetch::download(url, &downloaded)?;
    let extracted = work.join("extracted");
    archive::extract_tar(&downloaded, &extracted, false)?;
    match archive::only_directory(&extracted)? {
        Some(directory) => fs::rename(directory, checkout),
        None => fs::rename(extracted, checkout),
    }
}

fn transitive_dependencies(root: &Path) -> io::Result<Vec<Dependency>> {
    let mut dependencies = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(io::Error::other)?;
        if entry.file_name() == "build.zig.zon" {
            let content = fs::read_to_string(entry.path())?;
            for dependency in parse_zon(&content).unwrap_or_default() {
                if !dependencies
                    .iter()
                    .any(|known: &Dependency| known.hash == dependency.hash)
                {
                    dependencies.push(dependency);
                }
            }
        }
    }
    Ok(dependencies)
}

fn zig_hash(root: &Path) -> io::Result<String> {
    let mut hashes = BTreeMap::new();
    collect_hashes(root, root, &mut hashes)?;
    let mut combined = Sha256::new();
    for hash in hashes.values() {
        combined.update(hash);
    }
    Ok(format!("1220{:x}", combined.finalize()))
}

fn collect_hashes(
    root: &Path,
    directory: &Path,
    hashes: &mut BTreeMap<String, [u8; 32]>,
) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_hashes(root, &path, hashes)?;
            continue;
        }
        let relative = path.strip_prefix(root).map_err(io::Error::other)?;
        let name = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let value = if entry.file_type()?.is_symlink() {
            fs::read_link(&path)?.to_string_lossy().as_bytes().to_vec()
        } else {
            fs::read(&path)?
        };
        let mut hash = Sha256::new();
        hash.update(name.as_bytes());
        hash.update(b"\0\0");
        hash.update(value);
        hashes.insert(name, hash.finalize().into());
    }
    Ok(())
}

fn safe_hash(hash: &str) -> io::Result<()> {
    if crate::safe_token(hash, b"-_.") {
        Ok(())
    } else {
        Err(invalid(format!("unsafe Zig dependency hash {hash:?}")))
    }
}

#[cfg(test)]
#[path = "zig_tests.rs"]
mod tests;
