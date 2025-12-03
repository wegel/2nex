use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::manifest::*;
use crate::store::{encode_metadata_list, rewrite_branch_metadata};
use crate::utils::determine_category;

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
    println!("Creating bundle: {}", bundle_name);

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
    let repo = zub::Repo::open(Path::new(repo_path))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

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
    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // Attach bundle metadata (manifest hash, outputs) without changing the tree.
    rewrite_branch_metadata(repo_path, &bundle_branch, &metadata)?;

    Ok(())
}

pub fn fetch_and_verify_input(input_spec: &Source, download_dir: &str) -> io::Result<PathBuf> {
    println!("Fetching and verifying input: {:?}", input_spec);

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

        let mut file = fs::File::open(&resolved_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        let sha256_hash = hex::encode(Sha256::digest(&contents));
        if sha256_hash != input_spec.sha256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "SHA256 mismatch for local file '{}': expected {}, got {}",
                    file_path, input_spec.sha256, sha256_hash
                ),
            ));
        }

        println!("Local file verified: {}", file_path);
        return Ok(resolved_path.to_path_buf());
    }

    // for URL downloads, check cache first via hash symlink
    let hash_link_path = Path::new(download_dir).join(format!("sha256-{}", input_spec.sha256));
    if hash_link_path.exists() {
        // If the symbolic link exists, check that the target file also exists
        if let Ok(target_filename) = std::fs::read_link(&hash_link_path) {
            // Handle the relative path properly - the symlink points to a file in the same directory
            let full_target_path = Path::new(download_dir).join(&target_filename);
            if full_target_path.exists() {
                println!(
                    "Found existing file via hash link: {} -> {}",
                    hash_link_path.display(),
                    full_target_path.display()
                );

                // Always verify the hash even if found via symlink
                let mut file = fs::File::open(&full_target_path)?;
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;

                let calculated_hash = hex::encode(Sha256::digest(&contents));
                if calculated_hash == input_spec.sha256 {
                    println!("Hash verified for file found via symlink");
                    return Ok(full_target_path);
                } else {
                    println!(
                        "Hash mismatch for file found via symlink. Expected: {}, Got: {}",
                        input_spec.sha256, calculated_hash
                    );
                    println!("Removing invalid symlink: {}", hash_link_path.display());
                    std::fs::remove_file(&hash_link_path)?;
                    // Continue with normal download/verification process
                }
            } else {
                println!(
                    "Hash link target doesn't exist, removing stale link: {}",
                    hash_link_path.display()
                );
                std::fs::remove_file(&hash_link_path)?;
            }
        }
    }

    if let Some(url) = &input_spec.url {
        println!("Fetching input from URL: {}", url);

        // Extract a reasonable filename from the URL
        let url_path = url.split('/').last().unwrap_or("downloaded_file");
        let expected_filename = url_path.split('?').next().unwrap_or(url_path);
        let dst_path = Path::new(download_dir).join(expected_filename);

        // Download the file using curl
        if !dst_path.exists() {
            println!("Downloading {} using curl", dst_path.display());

            let status = std::process::Command::new("curl")
                .args([
                    "-L", // Follow redirects
                    "-f", // Fail on server errors
                    "-s", // Silent mode
                    "--output",
                    dst_path.to_str().unwrap(),
                    url,
                ])
                .status()?;

            if !status.success() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("curl download failed with status: {}", status),
                ));
            }
        } else {
            println!("File already exists: {}", dst_path.display());
        }

        // Verify the downloaded file
        let mut file = fs::File::open(&dst_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        // Check hash before creating the symlink
        let sha256_hash = hex::encode(Sha256::digest(&contents));
        if sha256_hash != input_spec.sha256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("SHA256 hash mismatch for downloaded input: {}", sha256_hash),
            ));
        }

        // Create a symbolic link from the hash to the file
        let hash_link_path = Path::new(download_dir).join(format!("sha256-{}", input_spec.sha256));
        if !hash_link_path.exists() {
            // Create relative path for the symlink to avoid including inputs_cache itself
            let filename = dst_path.file_name().unwrap();
            println!(
                "Creating hash symbolic link: {} -> {}",
                hash_link_path.display(),
                filename.to_string_lossy()
            );
            std::os::unix::fs::symlink(&filename, &hash_link_path)?;
        }

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

    // Collect all file paths
    for entry in WalkDir::new(output_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            file_paths.push(entry.path().to_path_buf());
        }
    }

    // Sort file paths to ensure consistent ordering
    file_paths.sort();

    let mut hasher = Sha256::new();

    for path in file_paths {
        // Update hasher with relative path
        let relative_path = path.strip_prefix(output_dir).unwrap();
        hasher.update(relative_path.to_string_lossy().as_bytes());
        hasher.update(b"\0"); // Use null byte as separator

        // Read and hash file contents
        let mut file = fs::File::open(&path)?;
        let mut buffer = [0; 4096];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        hasher.update(b"\0"); // Use null byte as separator between files
    }

    Ok(hex::encode(hasher.finalize()))
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

pub fn print_outputs(outputs: &HashMap<String, Vec<String>>) {
    println!("outputs:");
    let mut categories: Vec<_> = outputs.keys().collect();
    categories.sort();
    for category in categories {
        let files = outputs.get(category).unwrap();
        println!("  {}:", category);
        println!("    files:");
        for file in files {
            println!("      - {}", file);
        }
    }
}
