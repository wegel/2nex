//! Source input fetchers and checksum checks for package builds.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::manifest::Source;
use crate::utils::create_deterministic_tarball_uncompressed;

/// Fetch or verify a source input and return the local path to use for staging.
pub fn fetch_and_verify_input(input_spec: &Source, download_dir: &str) -> io::Result<PathBuf> {
    println!("Fetching and verifying input: {:?}", input_spec);

    if let Some(cargo_lock_ref) = &input_spec.cargo_lock {
        return fetch_cargo_lock(input_spec, cargo_lock_ref, download_dir);
    }
    if let Some(go_sum_ref) = &input_spec.go_sum {
        return fetch_go_sum(input_spec, go_sum_ref, download_dir);
    }
    if let Some(zig_zon_ref) = &input_spec.zig_zon {
        return fetch_zig_zon(input_spec, zig_zon_ref, download_dir);
    }
    if let Some(commit) = &input_spec.git_bundle {
        return fetch_git_bundle(input_spec, commit, download_dir);
    }
    if let Some(dev_path) = &input_spec.dev {
        return fetch_dev_source(input_spec, dev_path, download_dir);
    }
    if let Some(file_path) = &input_spec.file {
        return verify_local_file(input_spec, file_path, download_dir);
    }
    if let Some(url) = &input_spec.url {
        return fetch_url_source(input_spec, url, download_dir);
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "No URL or file path specified for input.",
    ))
}

/// Produce a Git bundle of one commit from the repository owning the manifest.
///
/// A bundle is a single file holding a header of refs and a packfile, and
/// `git clone <file> <dir>` treats it as a remote. Unlike a tarball of the
/// working tree it carries history, which manifests need: they name build
/// environments by historical blob, and a repository built from a snapshot
/// contains none of them.
///
/// Three details are not optional, each learned the hard way:
///
/// - `pack.threads=1`, or the output is not byte-reproducible. Three default
///   runs produce three different sha256 values at the same size; the
///   nondeterminism is parallel work-splitting, not content.
/// - a detached worktree, because `git bundle create <file> <commit-sha>`
///   refuses with "empty bundle": it needs a named ref, and a HEAD in a
///   detached worktree supplies one.
/// - an explicit commit rather than a branch, because bundling a branch whose
///   name differs from the repository's HEAD yields a clone with an empty tree
///   and `remote HEAD refers to nonexistent ref`.
fn fetch_git_bundle(
    input_spec: &Source,
    commit: &str,
    download_dir: &str,
) -> io::Result<PathBuf> {
    let expected = required_sha256(input_spec, "git_bundle")?;
    let bundle_path = Path::new(download_dir).join(format!("{}.bundle", input_spec.name));

    if bundle_path.exists() && file_sha256(&bundle_path)? == expected {
        println!("Found cached bundle: {}", bundle_path.display());
        return Ok(bundle_path);
    }

    let repository = repository_for_source(input_spec)?;
    let resolved = git_output(&repository, &["rev-parse", "--verify", &format!("{}^{{commit}}", commit)])?;

    let worktree = tempfile::tempdir()?;
    let worktree_path = worktree.path().join("tree");
    git_run(
        &repository,
        &[
            "worktree",
            "add",
            "--detach",
            "--quiet",
            &worktree_path.to_string_lossy(),
            &resolved,
        ],
    )?;

    let bundle_result = git_run(
        &worktree_path,
        &[
            "-c",
            "pack.threads=1",
            "bundle",
            "create",
            &bundle_path.to_string_lossy(),
            "HEAD",
        ],
    );
    let _ = git_run(
        &repository,
        &[
            "worktree",
            "remove",
            "--force",
            &worktree_path.to_string_lossy(),
        ],
    );
    bundle_result?;

    let actual = file_sha256(&bundle_path)?;
    if actual != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "bundle checksum mismatch for {}: expected {}, got {}",
                input_spec.name, expected, actual
            ),
        ));
    }

    println!("Created bundle: {} ({})", bundle_path.display(), resolved);
    Ok(bundle_path)
}

fn repository_for_source(input_spec: &Source) -> io::Result<PathBuf> {
    let start = match &input_spec.repository_snapshot {
        Some(snapshot) => snapshot.git_root.clone(),
        None => std::env::current_dir()?,
    };
    crate::manifest::repository_root_for_path(&start)
}

fn git_run(dir: &Path, args: &[&str]) -> io::Result<()> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "git {} failed in {}: {}",
        args.join(" "),
        dir.display(),
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

fn git_output(dir: &Path, args: &[&str]) -> io::Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git {} failed in {}: {}",
            args.join(" "),
            dir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn file_sha256(path: &Path) -> io::Result<String> {
    let contents = fs::read(path)?;
    Ok(hex::encode(Sha256::digest(&contents)))
}

fn fetch_cargo_lock(
    input_spec: &Source,
    cargo_lock_ref: &str,
    download_dir: &str,
) -> io::Result<PathBuf> {
    let cargo_lock_ref =
        materialize_repository_reference(input_spec, cargo_lock_ref, download_dir, "Cargo.lock")?;
    let cargo_toml_ref = input_spec
        .cargo_toml
        .as_deref()
        .map(|reference| {
            materialize_repository_reference(input_spec, reference, download_dir, "Cargo.toml")
        })
        .transpose()?;
    crate::cargo_vendor::vendor_from_lock(
        &cargo_lock_ref,
        cargo_toml_ref.as_deref(),
        required_sha256(input_spec, "cargo_lock")?,
        download_dir,
    )
}

fn fetch_go_sum(input_spec: &Source, go_sum_ref: &str, download_dir: &str) -> io::Result<PathBuf> {
    let go_sum_ref =
        materialize_repository_reference(input_spec, go_sum_ref, download_dir, "go.sum")?;
    crate::go_vendor::vendor_from_sum(
        &go_sum_ref,
        required_sha256(input_spec, "go_sum")?,
        download_dir,
    )
}

fn fetch_zig_zon(
    input_spec: &Source,
    zig_zon_ref: &str,
    download_dir: &str,
) -> io::Result<PathBuf> {
    let zig_zon_ref =
        materialize_repository_reference(input_spec, zig_zon_ref, download_dir, "build.zig.zon")?;
    crate::zig_vendor::vendor_from_zon(
        &zig_zon_ref,
        required_sha256(input_spec, "zig_zon")?,
        download_dir,
    )
}

fn fetch_dev_source(
    input_spec: &Source,
    dev_path: &str,
    download_dir: &str,
) -> io::Result<PathBuf> {
    println!("Creating dev tarball from: {}", dev_path);
    let source_dir = Path::new(dev_path);
    validate_dev_source(source_dir, dev_path)?;

    let tarball_path = Path::new(download_dir).join(format!("{}.tar", input_spec.name));
    if tarball_path.exists() {
        fs::remove_file(&tarball_path)?;
    }

    let prepare_script = source_dir.join(".nex-dev-prepare");
    if prepare_script.exists() && is_executable(&prepare_script) {
        run_prepare_script(source_dir, &prepare_script, &tarball_path)?;
    } else {
        create_deterministic_tarball_uncompressed(source_dir, &tarball_path)?;
    }

    println!("Dev tarball created: {}", tarball_path.display());
    Ok(tarball_path)
}

fn validate_dev_source(source_dir: &Path, dev_path: &str) -> io::Result<()> {
    if !source_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Dev source directory not found: {}", dev_path),
        ));
    }
    if !source_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Dev source must be a directory: {}", dev_path),
        ));
    }
    Ok(())
}

fn run_prepare_script(
    source_dir: &Path,
    prepare_script: &Path,
    tarball_path: &Path,
) -> io::Result<()> {
    let cwd = std::env::current_dir()?;
    let tarball_abs = cwd.join(tarball_path);
    let script_abs = cwd.join(prepare_script);
    let source_abs = cwd.join(source_dir);
    println!("Running .nex-dev-prepare: {}", prepare_script.display());

    if let Some(parent) = tarball_abs.parent() {
        fs::create_dir_all(parent)?;
    }
    let output = std::process::Command::new(&script_abs)
        .arg(&tarball_abs)
        .current_dir(&source_abs)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            ".nex-dev-prepare failed with status: {}\nstderr: {}",
            output.status, stderr
        )));
    }
    Ok(())
}

fn verify_local_file(
    input_spec: &Source,
    file_path: &str,
    download_dir: &str,
) -> io::Result<PathBuf> {
    if input_spec.repository_snapshot.is_some() {
        return materialize_repository_file(input_spec, file_path, download_dir);
    }

    println!("Verifying local file: {}", file_path);
    let resolved_path = Path::new(file_path);
    if !resolved_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Local file not found: {}", file_path),
        ));
    }

    let expected_sha256 = required_sha256(input_spec, &format!("file source: {}", file_path))?;
    let sha256_hash = sha256_file(resolved_path)?;
    if sha256_hash != expected_sha256 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "SHA256 mismatch for local file '{}': expected {}, got {}",
                file_path, expected_sha256, sha256_hash
            ),
        ));
    }

    println!("Local file verified: {}", file_path);
    Ok(resolved_path.to_path_buf())
}

fn fetch_url_source(input_spec: &Source, url: &str, download_dir: &str) -> io::Result<PathBuf> {
    let expected_sha256 = required_sha256(input_spec, &format!("url source: {}", url))?;
    let dst_path = Path::new(download_dir).join(expected_sha256);

    if dst_path.exists() {
        verify_download(&dst_path, expected_sha256)?;
        println!("Verified cache hit: {}", expected_sha256);
        return Ok(dst_path);
    }

    println!("Downloading from {}", url);
    let tmp_path = Path::new(download_dir).join(format!("{}.tmp", expected_sha256));
    download_to_temp_file(url, &tmp_path)?;
    verify_download(&tmp_path, expected_sha256)?;
    std::fs::rename(&tmp_path, &dst_path)?;
    println!("Cached as {}", expected_sha256);

    Ok(dst_path)
}

fn materialize_repository_file(
    input_spec: &Source,
    file_path: &str,
    download_dir: &str,
) -> io::Result<PathBuf> {
    let snapshot = input_spec.repository_snapshot.as_ref().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "missing repository snapshot")
    })?;
    let expected_sha256 = required_sha256(input_spec, &format!("file source: {}", file_path))?;
    let bytes =
        crate::utils::fetch_git_file(&snapshot.git_root, &snapshot.revision, Path::new(file_path))?;
    let actual_sha256 = hex::encode(Sha256::digest(&bytes));
    if actual_sha256 != expected_sha256 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "SHA256 mismatch for '{}' at repository revision {}: expected {}, got {}",
                file_path, snapshot.revision, expected_sha256, actual_sha256
            ),
        ));
    }

    let cache_path = Path::new(download_dir).join(expected_sha256);
    write_atomic(&cache_path, &bytes)?;
    Ok(cache_path)
}

fn materialize_repository_reference(
    input_spec: &Source,
    reference: &str,
    download_dir: &str,
    label: &str,
) -> io::Result<String> {
    if reference.starts_with("http://") || reference.starts_with("https://") {
        return Ok(reference.to_string());
    }
    let Some(snapshot) = input_spec.repository_snapshot.as_ref() else {
        return Ok(reference.to_string());
    };

    let bytes =
        crate::utils::fetch_git_file(&snapshot.git_root, &snapshot.revision, Path::new(reference))?;
    let cache_key = hex::encode(Sha256::digest(
        format!("{}\0{}", snapshot.revision, reference).as_bytes(),
    ));
    let cache_path = Path::new(download_dir).join(format!("repository-{}-{}", cache_key, label));
    write_atomic(&cache_path, &bytes)?;
    Ok(cache_path.to_string_lossy().into_owned())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("cache path has no parent: {}", path.display()),
        )
    })?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.flush()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn download_to_temp_file(url: &str, tmp_path: &Path) -> io::Result<()> {
    let status = std::process::Command::new("curl")
        .args([
            "-L",
            "-f",
            "-s",
            "--output",
            tmp_path
                .to_str()
                .expect("temporary download path should be UTF-8"),
            url,
        ])
        .status()?;

    if status.success() {
        Ok(())
    } else {
        let _ = std::fs::remove_file(tmp_path);
        Err(io::Error::other(format!(
            "curl download failed with status: {}",
            status
        )))
    }
}

fn verify_download(tmp_path: &Path, expected_sha256: &str) -> io::Result<()> {
    let sha256_hash = sha256_file(tmp_path)?;
    if sha256_hash == expected_sha256 {
        return Ok(());
    }

    let _ = std::fs::remove_file(tmp_path);
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "SHA256 mismatch: expected {}, got {}",
            expected_sha256, sha256_hash
        ),
    ))
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(hex::encode(Sha256::digest(&contents)))
}

fn required_sha256<'a>(input_spec: &'a Source, source_kind: &str) -> io::Result<&'a str> {
    input_spec.sha256.as_deref().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("sha256 required for {}", source_kind),
        )
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    false
}
