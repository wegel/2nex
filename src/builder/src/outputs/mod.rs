use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use walkdir::WalkDir;

use crate::manifest::*;
use crate::ostree::*;
use crate::runtime::scanner::RuntimeScanResult;
use crate::utils::determine_category;

pub fn output_branch_metadata(
    manifest: &Manifest,
    spec: &OutputSpec,
    manifest_hash: &str,
) -> io::Result<Vec<(String, String)>> {
    let mut metadata = Vec::new();
    metadata.push(("nex.manifest.hash".to_string(), manifest_hash.to_string()));
    if let Some(checksum) = &manifest.package.checksum {
        metadata.push(("nex.build.checksum".to_string(), checksum.clone()));
    }
    if let Some(encoded) = encode_metadata_list(&spec.requires)? {
        metadata.push(("nex.output.requires".to_string(), encoded));
    }
    if let Some(encoded) = encode_metadata_list(&spec.suggests)? {
        metadata.push(("nex.output.suggests".to_string(), encoded));
    }
    Ok(metadata)
}

pub fn bundle_branch_metadata(
    manifest: &Manifest,
    bundle: &Bundle,
    manifest_hash: &str,
) -> io::Result<Vec<(String, String)>> {
    let mut metadata = Vec::new();
    metadata.push(("nex.manifest.hash".to_string(), manifest_hash.to_string()));
    if let Some(checksum) = &manifest.package.checksum {
        metadata.push(("nex.build.checksum".to_string(), checksum.clone()));
    }
    if let Some(encoded) = encode_metadata_list(&bundle.requires)? {
        metadata.push(("nex.bundle.requires".to_string(), encoded));
    }
    if let Some(encoded) = encode_metadata_list(&bundle.suggests)? {
        metadata.push(("nex.bundle.suggests".to_string(), encoded));
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

    let temp_dir = TempDir::new()?;
    let temp_dir_path = temp_dir.path();

    for output in &bundle.includes {
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, output
        );
        checkout_ostree_into(repo_path, &branch_name, temp_dir_path, true, false)?;
    }

    let bundle_branch = format!(
        "x86_64/{}/{}/{}/bundles/{}",
        manifest.package.slug, manifest.package.version, manifest.package.flavor, bundle_name
    );

    let metadata = bundle_branch_metadata(manifest, bundle, manifest_hash)?;

    commit_to_ostree(repo_path, &bundle_branch, temp_dir_path, &metadata)?;

    Ok(())
}

pub fn fetch_and_verify_input(input_spec: &Source, download_dir: &str) -> io::Result<PathBuf> {
    println!("Fetching and verifying input: {:?}", input_spec);

    // First check if we have a symbolic link with the hash name
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
    } else if let Some(file_path) = &input_spec.file {
        println!("Fetching input from local file: {}", file_path);

        let candidate_path = Path::new(file_path);
        let mut resolved_path = if candidate_path.is_absolute() {
            candidate_path.to_path_buf()
        } else if candidate_path.exists() {
            candidate_path.to_path_buf()
        } else {
            Path::new("./inputs_cache").join(candidate_path)
        };
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
                format!("SHA256 hash mismatch for local input: {}", sha256_hash),
            ));
        }

        let download_dir_path = Path::new(download_dir);
        let staged_path = {
            let cwd = env::current_dir().expect("Failed to determine current directory");
            let resolved_abs = if resolved_path.is_absolute() {
                resolved_path.clone()
            } else {
                cwd.join(&resolved_path)
            };
            let download_abs = if download_dir_path.is_absolute() {
                download_dir_path.to_path_buf()
            } else {
                cwd.join(download_dir_path)
            };
            if resolved_abs.starts_with(&download_abs) {
                resolved_path.clone()
            } else {
                let file_name = resolved_path.file_name().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Local file must have a valid filename",
                    )
                })?;
                let target_path = download_dir_path.join(file_name);
                if !target_path.exists() {
                    fs::copy(&resolved_path, &target_path)?;
                }
                target_path
            }
        };

        resolved_path = staged_path;

        // Create a symbolic link from the hash to the file
        let hash_link_path = Path::new(download_dir).join(format!("sha256-{}", input_spec.sha256));
        if !hash_link_path.exists() {
            // Create relative path for the symlink to avoid including inputs_cache itself
            let filename = resolved_path.file_name().unwrap();
            println!(
                "Creating hash symbolic link: {} -> {}",
                hash_link_path.display(),
                filename.to_string_lossy()
            );
            std::os::unix::fs::symlink(&filename, &hash_link_path)?;
        }

        Ok(resolved_path)
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

pub fn print_outputs(
    outputs: &HashMap<String, Vec<String>>,
    runtime_suggestions: Option<&RuntimeScanResult>,
    verbose_reasons: bool,
) {
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
        if let Some(suggestions) = runtime_suggestions {
            if let Some(commits) = suggestions.category_resolved(category) {
                println!("    requires:");
                for (commit, reasons) in commits {
                    println!("      - {}", commit);
                    if verbose_reasons {
                        for reason in reasons {
                            println!("        # {}", reason);
                        }
                    }
                }
            }
            if let Some(unresolved) = suggestions.category_unresolved(category) {
                println!("    unresolved:");
                for (req, reasons) in unresolved {
                    println!("      - {}", req);
                    if verbose_reasons {
                        for reason in reasons {
                            println!("        # {}", reason);
                        }
                    }
                }
            }
        }
    }
}

