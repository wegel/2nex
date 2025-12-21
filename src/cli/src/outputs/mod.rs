use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::manifest::*;
use crate::store::{create_artifact, encode_metadata_list, get_branch_tree, rewrite_branch_metadata};
use crate::utils::{create_deterministic_tarball_uncompressed, determine_category};

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    false
}

pub fn output_branch_metadata(
    manifest: &Manifest,
    _spec: &OutputSpec,
    manifest_hash: &str,
) -> io::Result<Vec<(String, String)>> {
    let mut metadata = Vec::new();
    metadata.push(("nex.manifest.hash".to_string(), manifest_hash.to_string()));
    if let Some(checksum) = &manifest.package.checksum {
        metadata.push(("nex.build.checksum".to_string(), checksum.clone()));
    }
    Ok(metadata)
}

pub fn bundle_branch_metadata(
    manifest: &Manifest,
    _bundle: &Bundle,
    manifest_hash: &str,
) -> io::Result<Vec<(String, String)>> {
    let mut metadata = Vec::new();
    metadata.push(("nex.manifest.hash".to_string(), manifest_hash.to_string()));
    if let Some(checksum) = &manifest.package.checksum {
        metadata.push(("nex.build.checksum".to_string(), checksum.clone()));
    }
    Ok(metadata)
}

pub fn commit_bundle(
    repo_path: &str,
    bundle_name: &str,
    bundle: &Bundle,
    manifest: &Manifest,
    manifest_hash: &str,
) -> io::Result<()> {
    // collect output commit refs
    let mut output_commits = Vec::new();

    for output in &bundle.includes {
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.namespace_path(),
            manifest.package.slug,
            manifest.package.version,
            output
        );
        output_commits.push(branch_name);
    }

    let bundle_branch = format!(
        "x86_64/{}/{}/{}/bundles/{}",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version,
        bundle_name
    );

    let mut metadata = bundle_branch_metadata(manifest, bundle, manifest_hash)?;

    // add bundle outputs metadata
    if let Some(encoded) = encode_metadata_list(&output_commits)? {
        metadata.push(("nex.bundle.outputs".to_string(), encoded));
    }

    // Build bundle commit by merging output commits directly in the store.
    // Use "last wins" semantics to match union checkouts during bundling.
    let repo =
        zub::Repo::open(Path::new(repo_path)).map_err(|e| io::Error::other(e.to_string()))?;

    let ref_strs: Vec<&str> = output_commits.iter().map(|s| s.as_str()).collect();
    zub::ops::union_trees(
        &repo,
        &ref_strs,
        &bundle_branch,
        zub::ops::UnionOptions {
            on_conflict: zub::ops::ConflictResolution::Last,
            ..Default::default()
        },
    )
    .map_err(|e| io::Error::other(e.to_string()))?;

    // Attach bundle metadata (manifest hash, outputs) without changing the tree.
    rewrite_branch_metadata(repo_path, &bundle_branch, &metadata)?;

    // create artifact for this bundle
    let tree_hash = get_branch_tree(repo_path, &bundle_branch)?;
    let artifact_output = format!("bundles/{}", bundle_name);
    create_artifact(repo_path, &tree_hash, manifest_hash, &artifact_output)?;

    Ok(())
}

pub fn fetch_and_verify_input(input_spec: &Source, download_dir: &str) -> io::Result<PathBuf> {
    println!("Fetching and verifying input: {:?}", input_spec);

    // handle cargo_lock source type (automatic vendoring)
    if let Some(cargo_lock_ref) = &input_spec.cargo_lock {
        let expected_sha256 = input_spec.sha256.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "sha256 required for cargo_lock source",
            )
        })?;
        return crate::cargo_vendor::vendor_from_lock(
            cargo_lock_ref,
            input_spec.cargo_toml.as_deref(),
            expected_sha256,
            download_dir,
        );
    }

    // handle go_sum source type (automatic Go module vendoring)
    if let Some(go_sum_ref) = &input_spec.go_sum {
        let expected_sha256 = input_spec.sha256.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "sha256 required for go_sum source",
            )
        })?;
        return crate::go_vendor::vendor_from_sum(go_sum_ref, expected_sha256, download_dir);
    }

    // handle zig_zon source type (automatic Zig dependency vendoring)
    if let Some(zig_zon_ref) = &input_spec.zig_zon {
        let expected_sha256 = input_spec.sha256.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "sha256 required for zig_zon source",
            )
        })?;
        return crate::zig_vendor::vendor_from_zon(zig_zon_ref, expected_sha256, download_dir);
    }

    // handle dev source type (local directory for development)
    if let Some(dev_path) = &input_spec.dev {
        println!("Creating dev tarball from: {}", dev_path);

        let source_dir = Path::new(dev_path);
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

        let tarball_name = format!("{}.tar", input_spec.name);
        let tarball_path = Path::new(download_dir).join(&tarball_name);

        // always recreate dev tarball (no caching - source may have changed)
        if tarball_path.exists() {
            fs::remove_file(&tarball_path)?;
        }

        // check for .nex-dev-prepare executable
        let prepare_script = source_dir.join(".nex-dev-prepare");
        if prepare_script.exists() && is_executable(&prepare_script) {
            // use absolute paths since script runs with different cwd
            let cwd = std::env::current_dir()?;
            let tarball_abs = cwd.join(&tarball_path);
            let script_abs = cwd.join(&prepare_script);
            let source_abs = cwd.join(source_dir);
            println!("Running .nex-dev-prepare: {}", prepare_script.display());
            // ensure parent directory exists
            if let Some(parent) = tarball_abs.parent() {
                fs::create_dir_all(parent)?;
            }
            let output = std::process::Command::new(&script_abs)
                .arg(&tarball_abs)
                .current_dir(&source_abs)
                .output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        ".nex-dev-prepare failed with status: {}\nstderr: {}",
                        output.status, stderr
                    ),
                ));
            }
        } else {
            // default: create uncompressed tarball from directory
            create_deterministic_tarball_uncompressed(source_dir, &tarball_path)?;
        }

        println!("Dev tarball created: {}", tarball_path.display());
        return Ok(tarball_path);
    }

    // for local files, verify directly from source - no caching
    if let Some(file_path) = &input_spec.file {
        println!("Verifying local file: {}", file_path);

        let resolved_path = Path::new(file_path);
        if !resolved_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Local file not found: {}", file_path),
            ));
        }

        let expected_sha256 = input_spec.sha256.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("sha256 required for file source: {}", file_path),
            )
        })?;

        let mut file = fs::File::open(resolved_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        let sha256_hash = hex::encode(Sha256::digest(&contents));
        if sha256_hash != *expected_sha256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "SHA256 mismatch for local file '{}': expected {}, got {}",
                    file_path, expected_sha256, sha256_hash
                ),
            ));
        }

        println!("Local file verified: {}", file_path);
        return Ok(resolved_path.to_path_buf());
    }

    // for URL downloads, use content hash as filename (content-addressable cache)
    if let Some(url) = &input_spec.url {
        let expected_sha256 = input_spec.sha256.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("sha256 required for url source: {}", url),
            )
        })?;

        let dst_path = Path::new(download_dir).join(expected_sha256);

        if dst_path.exists() {
            println!("Cache hit: {}", expected_sha256);
            return Ok(dst_path);
        }

        println!("Downloading from {}", url);

        // download to a temp file first, then rename after verification
        let tmp_path = Path::new(download_dir).join(format!("{}.tmp", expected_sha256));

        let status = std::process::Command::new("curl")
            .args([
                "-L", // follow redirects
                "-f", // fail on server errors
                "-s", // silent mode
                "--output",
                tmp_path.to_str().unwrap(),
                url,
            ])
            .status()?;

        if !status.success() {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(io::Error::other(format!(
                "curl download failed with status: {}",
                status
            )));
        }

        // verify the downloaded file
        let mut file = fs::File::open(&tmp_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        let sha256_hash = hex::encode(Sha256::digest(&contents));
        if sha256_hash != *expected_sha256 {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "SHA256 mismatch: expected {}, got {}",
                    expected_sha256, sha256_hash
                ),
            ));
        }

        // rename to final content-addressed filename
        std::fs::rename(&tmp_path, &dst_path)?;
        println!("Cached as {}", expected_sha256);

        Ok(dst_path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No URL or file path specified for input.",
        ))
    }
}

pub fn calculate_output_checksum(output_dir: &Path) -> io::Result<String> {
    let mut file_paths: Vec<PathBuf> = Vec::new();

    // collect all file paths
    for entry in WalkDir::new(output_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            file_paths.push(entry.path().to_path_buf());
        }
    }

    // sort for consistent ordering
    file_paths.sort();

    // hash each file in parallel using BLAKE3
    let file_hashes: Vec<io::Result<([u8; 32], String)>> = file_paths
        .par_iter()
        .map(|path| {
            let relative_path = path
                .strip_prefix(output_dir)
                .unwrap()
                .to_string_lossy()
                .into_owned();

            let mut hasher = blake3::Hasher::new();
            let mut file = fs::File::open(path)?;
            let mut buffer = [0u8; 65536]; // 64KB buffer
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hasher.update(&buffer[..count]);
            }
            Ok((*hasher.finalize().as_bytes(), relative_path))
        })
        .collect();

    // combine all file hashes deterministically (must be in sorted order)
    let mut final_hasher = blake3::Hasher::new();
    for result in file_hashes {
        let (hash, path) = result?;
        final_hasher.update(path.as_bytes());
        final_hasher.update(b"\0");
        final_hasher.update(&hash);
        final_hasher.update(b"\0");
    }

    Ok(final_hasher.finalize().to_hex().to_string())
}

pub fn categorize_files(rootfs_dir: &Path) -> HashMap<String, Vec<String>> {
    let mut outputs = HashMap::new();

    for entry in WalkDir::new(rootfs_dir) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() || entry.file_type().is_symlink() {
            let relative_path = entry.path().strip_prefix(rootfs_dir).unwrap();
            let relative_path_str = format!("/{}", relative_path.to_str().unwrap());

            let category = determine_category(&relative_path_str);
            outputs
                .entry(category)
                .or_insert_with(Vec::new)
                .push(relative_path_str);
        }
    }

    // Sort each vector of paths in the HashMap
    for paths in outputs.values_mut() {
        paths.sort();
    }

    outputs
}

