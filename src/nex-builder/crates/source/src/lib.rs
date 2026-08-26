//! Verified source acquisition and deterministic dependency archives.

mod archive;
mod cache;
mod cargo;
mod fetch;
mod go;
mod zig;

pub use fetch::local_path;

use std::io;
use std::path::{Path, PathBuf};

/// One source form accepted by a Nex manifest.
#[derive(Clone, Copy, Debug)]
pub enum SourceKind<'a> {
    /// Download one ready-to-stage file.
    Url(&'a str),
    /// Verify one file from the manifest's Git tree.
    File(&'a Path),
    /// Create a deterministic bundle containing one Git commit.
    GitBundle(&'a str),
    /// Create a Cargo vendor archive from a Cargo.lock reference.
    CargoLock(&'a str),
    /// Create a Go vendor archive from a go.sum reference.
    GoSum(&'a str),
    /// Create a Zig package-cache archive from a build.zig.zon reference.
    ZigZon(&'a str),
}

/// Everything needed to produce one verified source file.
#[derive(Clone, Copy, Debug)]
pub struct SourceRequest<'a> {
    /// Source form and its input reference.
    pub kind: SourceKind<'a>,
    /// SHA-256 of the exact file passed to the build script.
    pub sha256: &'a str,
    /// Manifest that owns relative paths and Git bundle commits.
    pub manifest: &'a Path,
    /// Persistent source and download cache.
    pub cache: &'a Path,
}

/// Return one verified local file, creating and caching it when absent.
pub fn acquire(request: SourceRequest<'_>) -> io::Result<PathBuf> {
    validate_hash(request.sha256)?;
    if let SourceKind::File(path) = request.kind {
        let path = fetch::local_path(request.manifest, path);
        cache::verify(&path, request.sha256)?;
        return Ok(path);
    }

    let output = output_path(&request);
    cache::materialize(&output, request.sha256, |temporary| match request.kind {
        SourceKind::Url(url) => fetch::download(url, temporary),
        SourceKind::GitBundle(commit) => fetch::git_bundle(request.manifest, commit, temporary),
        SourceKind::CargoLock(reference) => {
            cargo::vendor(reference, request.manifest, request.cache, temporary)
        }
        SourceKind::GoSum(reference) => {
            go::vendor(reference, request.manifest, request.cache, temporary)
        }
        SourceKind::ZigZon(reference) => {
            zig::vendor(reference, request.manifest, request.cache, temporary)
        }
        SourceKind::File(_) => unreachable!("local files return before cache acquisition"),
    })?;
    Ok(output)
}

fn output_path(request: &SourceRequest<'_>) -> PathBuf {
    let leaf = format!("sha256-{}", request.sha256);
    match request.kind {
        SourceKind::CargoLock(_) => request
            .cache
            .join("cargo_vendor")
            .join(format!("{leaf}.tar.gz")),
        SourceKind::GoSum(_) => request
            .cache
            .join("go_vendor")
            .join(format!("{leaf}.tar.gz")),
        SourceKind::ZigZon(_) => request
            .cache
            .join("zig_vendor")
            .join(format!("{leaf}.tar.gz")),
        _ => request.cache.join(leaf),
    }
}

fn validate_hash(hash: &str) -> io::Result<()> {
    if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid SHA-256 {hash:?}"),
        ))
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn safe_token(value: &str, punctuation: &[u8]) -> bool {
    !matches!(value, "" | "." | "..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || punctuation.contains(&byte))
}
