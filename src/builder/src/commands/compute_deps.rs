//! compute-deps command: compute and store runtime dependencies in manifest.
//!
//! This command scans ELF files in a built package's outputs and populates
//! the manifest's `resolution` map, mapping file paths to dependency names.
//! Internal libraries (provided by the package itself) are marked with `@self`.
//!
//! This eliminates the need for runtime ELF scanning during `nex install`.

use clap::Args;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use goblin::Object;
use tempfile::TempDir;

use crate::manifest::parser::load_manifest_from_source;
use crate::manifest::types::FileEntry;
use crate::manifest::types::ManifestSource;
use crate::ostree::commit_to_ostree;
use crate::ostree_native::OstreeRepo;
use crate::repo::resolve_repo_path;
use crate::utils::hash_file_content;

#[derive(Args)]
pub struct ComputeDepsArgs {
    /// Path to the package manifest (e.g., pkg/cli/shells/bash/bash.yaml)
    pub manifest: String,

    /// OSTree repository path (auto-detected if not specified)
    #[clap(long)]
    pub repo: Option<String>,

    /// Don't modify the manifest, just print what would be computed
    #[clap(long)]
    pub dry_run: bool,

    /// Show verbose output
    #[clap(long, short)]
    pub verbose: bool,
}

pub fn run(args: &ComputeDepsArgs) -> io::Result<()> {
    let repo_path = resolve_repo_path(args.repo.as_deref())?;

    // load the manifest
    let manifest_path = Path::new(&args.manifest);
    if !manifest_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Manifest not found: {}", args.manifest),
        ));
    }

    let manifest_data =
        load_manifest_from_source(&ManifestSource::Path(manifest_path.to_path_buf()))?;
    let manifest = match manifest_data {
        crate::manifest::types::ManifestData::Package(m) => m,
        crate::manifest::types::ManifestData::System(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "compute-deps only works on package manifests, not system manifests",
            ));
        }
    };

    compute_deps_for_manifest(
        &manifest,
        &repo_path,
        manifest_path,
        args.verbose,
        args.dry_run,
    )?;

    // refresh OSTree metadata if not dry-run (manifest was updated)
    if !args.dry_run {
        crate::refresh_package_metadata(&repo_path, &manifest, manifest_path)?;
    }

    Ok(())
}

/// Compute and store runtime dependencies for a package manifest.
/// This is called after outputs are committed to OSTree.
/// - Scans ELF files for DT_NEEDED dependencies
/// - Builds resolution map (file_path -> dependency_name)
/// - Creates {hash}/files union commit
/// - Updates manifest with resolution map and needs lists
pub fn compute_deps_for_manifest(
    manifest: &crate::manifest::types::Manifest,
    repo_path: &str,
    manifest_path: &Path,
    verbose: bool,
    dry_run: bool,
) -> io::Result<()> {
    println!(
        "Computing dependencies for {}/{}...",
        manifest.package.namespace, manifest.package.slug
    );

    // collect build dependencies from manifest (these are what we resolve against)
    let dep_commits: Vec<String> = manifest
        .dependencies
        .iter()
        .map(|d| d.commit.clone())
        .collect();
    if verbose {
        println!("  Build dependencies: {} commits", dep_commits.len());
    }

    // build provider_key -> dependency_name mapping
    // this maps e.g. "libs/system/glibc/2.39" -> "glibc"
    let mut provider_key_to_dep_name: HashMap<String, String> = HashMap::new();
    // track which provider_keys use each name (to detect conflicts across different packages)
    let mut dep_name_to_providers: HashMap<String, HashSet<String>> = HashMap::new();

    for dep in &manifest.dependencies {
        let provider_key = extract_provider_key(&dep.commit);
        if let Some(name) = &dep.name {
            provider_key_to_dep_name.insert(provider_key.clone(), name.clone());
            dep_name_to_providers
                .entry(name.clone())
                .or_default()
                .insert(provider_key);
        } else {
            // dependency without a name - error out
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Dependency {} has no name field. All dependencies must have unique names.",
                    dep.commit
                ),
            ));
        }
    }

    // validate: same name should only map to ONE provider_key (package)
    // multiple commits from the same package sharing a name is OK
    let conflicts: Vec<_> = dep_name_to_providers
        .iter()
        .filter(|(_, providers)| providers.len() > 1)
        .collect();
    if !conflicts.is_empty() {
        let mut err_msg =
            String::from("Dependency name conflicts (same name used for different packages):\n");
        for (name, providers) in conflicts {
            err_msg.push_str(&format!("  '{}' maps to:\n", name));
            for provider in providers {
                err_msg.push_str(&format!("    - {}\n", provider));
            }
        }
        return Err(io::Error::new(io::ErrorKind::InvalidInput, err_msg));
    }

    if verbose {
        println!(
            "  Provider key mappings: {}",
            provider_key_to_dep_name.len()
        );
    }

    // find the outputs for this package in OSTree
    let arch = "x86_64";
    let output_prefix = format!(
        "{}/{}/{}/{}/outputs",
        arch,
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version
    );

    let repo = OstreeRepo::open(&repo_path)?;
    let all_refs = repo.refs(Some(&output_prefix))?;
    if all_refs.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "No outputs found for package. Build it first. Looked for: {}",
                output_prefix
            ),
        ));
    }

    // build provider key for this package (used for internal library resolution)
    let self_provider_key = format!(
        "{}/{}/{}",
        manifest.package.namespace, manifest.package.slug, manifest.package.version
    );

    // build provider lookup from dependencies + self outputs (for internal libs)
    let provider_lookup = build_provider_lookup(
        repo_path,
        &dep_commits,
        &all_refs,
        &self_provider_key,
        verbose,
    )?;
    if verbose {
        println!("  Provider index: {} libraries", provider_lookup.len());
    }

    // process each output
    let mut new_outputs: HashMap<String, Vec<FileEntry>> = HashMap::new();
    let mut resolution: HashMap<String, String> = HashMap::new();

    for output_ref in &all_refs {
        // extract output name from ref (e.g., "x86_64/.../outputs/bin" -> "bin")
        let output_name = output_ref.split('/').last().unwrap_or("unknown");
        if output_name == "discard" {
            continue;
        }

        println!("  Processing output: {}", output_name);

        // checkout the output to scan it
        // create temp dir inside repo to stay on same filesystem (hardlinks)
        let repo_tmp = PathBuf::from(repo_path).join("tmp");
        fs::create_dir_all(&repo_tmp)?;
        let temp = TempDir::new_in(&repo_tmp)?;
        repo.checkout(output_ref, temp.path(), true)?;

        // get files from manifest for this output (if exists)
        let manifest_files: Vec<String> = manifest
            .outputs
            .get(output_name)
            .map(|spec| spec.files.iter().map(|f| f.path.clone()).collect())
            .unwrap_or_default();

        // if manifest doesn't list files, scan the checkout
        let files_to_process = if manifest_files.is_empty() {
            scan_files_in_checkout(temp.path())?
        } else {
            manifest_files
        };

        let mut file_entries: Vec<FileEntry> = Vec::new();

        for file_path in &files_to_process {
            let physical_path = temp.path().join(file_path.trim_start_matches('/'));
            if !physical_path.exists() {
                if verbose {
                    println!("    Skipping missing file: {}", file_path);
                }
                continue;
            }

            // scan for DT_NEEDED
            let needed = scan_elf_for_deps(&physical_path)?;
            if needed.is_empty() {
                // no deps (script or static binary)
                file_entries.push(FileEntry {
                    path: file_path.clone(),
                    needs: Vec::new(),
                });
                continue;
            }

            if verbose {
                println!("    {}: {} dependencies", file_path, needed.len());
            }

            // resolve each needed library
            let mut needs: Vec<String> = Vec::new();
            for lib_name in &needed {
                if let Some((provider_key, lib_path, _files_commit)) = provider_lookup.get(lib_name)
                {
                    needs.push(lib_path.clone());

                    // resolve provider_key to dependency name (or @self for internal libs)
                    let resolution_value = if provider_key == &self_provider_key {
                        "@self".to_string()
                    } else if let Some(dep_name) = provider_key_to_dep_name.get(provider_key) {
                        dep_name.clone()
                    } else {
                        // unresolved provider - shouldn't happen if deps are correct
                        println!(
                            "    Warning: no dependency found for provider {}",
                            provider_key
                        );
                        provider_key.clone()
                    };

                    // add to resolution map (file_path -> dep_name or @self)
                    resolution
                        .entry(lib_path.clone())
                        .or_insert(resolution_value.clone());

                    if verbose {
                        println!("      {} -> {} ({})", lib_name, resolution_value, lib_path);
                    }
                } else if verbose {
                    println!("      {} -> (unresolved)", lib_name);
                }
            }

            file_entries.push(FileEntry {
                path: file_path.clone(),
                needs,
            });
        }

        new_outputs.insert(output_name.to_string(), file_entries);
    }

    // print summary
    println!();
    println!("Computed dependencies:");
    println!("  {} outputs processed", new_outputs.len());
    println!("  {} unique file resolutions", resolution.len());

    if dry_run {
        println!();
        println!("Resolution map (file_path -> dependency_name):");
        for (file_path, dep_name) in &resolution {
            println!("  {} -> {}", file_path, dep_name);
        }
        println!();
        println!("(dry run - manifest not modified)");
        return Ok(());
    }

    // create files commit for this package (union of all outputs)
    println!();
    println!(
        "Creating files commit for {}/{}...",
        manifest.package.namespace, manifest.package.slug
    );

    // determine address hash: use checksum if stable, else use manifest git blob SHA
    let has_stable_checksum =
        manifest.package.checksum.is_some() && manifest.package.stable_checksum.unwrap_or(true);

    let address_hash = if has_stable_checksum {
        manifest.package.checksum.clone().unwrap()
    } else {
        // use manifest's git blob SHA (input-addressed for bootstrap packages)
        hash_file_content(manifest_path)?
    };

    let files_commit = create_files_commit(
        repo_path,
        &all_refs,
        &address_hash,
        &manifest.package.namespace,
        &manifest.package.slug,
        &manifest.package.version,
    )?;
    println!("  Created: {}", files_commit);

    // update the manifest file
    update_manifest_file(manifest_path, &new_outputs, &resolution)?;

    println!();
    println!("Manifest updated: {}", manifest_path.display());

    Ok(())
}

/// Build a lookup table from library basename to (provider_key, file_path, files_commit).
/// Uses reverse-order resolution: last dependency in list wins.
/// Also creates/finds files commits for each dependency package.
/// Includes the current package's own outputs for internal library resolution.
fn build_provider_lookup(
    repo_path: &str,
    dep_commits: &[String],
    self_output_refs: &[String],
    self_provider_key: &str,
    verbose: bool,
) -> io::Result<HashMap<String, (String, String, String)>> {
    let repo = OstreeRepo::open(repo_path)?;
    let mut lookup: HashMap<String, (String, String, String)> = HashMap::new();

    // group commits by package (provider_key) to find/create files commits
    let mut packages: HashMap<String, Vec<String>> = HashMap::new();
    for commit in dep_commits {
        let provider_key = extract_provider_key(commit);
        packages
            .entry(provider_key)
            .or_default()
            .push(commit.clone());
    }

    // find or create files commit for each package
    let mut files_commits: HashMap<String, String> = HashMap::new();
    for (provider_key, commits) in &packages {
        // check if files commit already exists for this package
        let files_commit = find_or_create_files_commit(repo_path, provider_key, commits, verbose)?;
        files_commits.insert(provider_key.clone(), files_commit);
    }

    // process in REVERSE order - last dep wins (matches --union checkout behavior)
    for commit in dep_commits.iter().rev() {
        let provider_key = extract_provider_key(commit);
        let files_commit = files_commits
            .get(&provider_key)
            .cloned()
            .unwrap_or_else(|| commit.clone());

        // list files in this commit
        let files = match repo.ls(commit) {
            Ok(f) => f,
            Err(e) => {
                if verbose {
                    println!("    Warning: Could not list {}: {}", commit, e);
                }
                continue;
            }
        };

        // index each file by basename
        for file in files {
            if file.ends_with('/') || file.is_empty() {
                continue; // skip directories
            }

            let file_path = if file.starts_with('/') {
                file.clone()
            } else {
                format!("/{}", file)
            };

            // only index library-like files
            if !is_library_path(&file_path) {
                continue;
            }

            if let Some(basename) = Path::new(&file_path).file_name().and_then(|n| n.to_str()) {
                // reverse order: don't overwrite existing entries (earlier in reverse = later in original)
                lookup.entry(basename.to_string()).or_insert_with(|| {
                    (
                        provider_key.clone(),
                        file_path.clone(),
                        files_commit.clone(),
                    )
                });
            }
        }
    }

    // index current package's own outputs (highest priority - overwrites deps)
    // this allows internal libraries like libsystemd-shared-257.so to be resolved
    let self_files_commit =
        find_or_create_files_commit(repo_path, self_provider_key, self_output_refs, verbose)?;

    for output_ref in self_output_refs {
        let files = match repo.ls(output_ref) {
            Ok(f) => f,
            Err(e) => {
                if verbose {
                    println!(
                        "    Warning: Could not list self output {}: {}",
                        output_ref, e
                    );
                }
                continue;
            }
        };

        for file in files {
            if file.ends_with('/') || file.is_empty() {
                continue;
            }

            let file_path = if file.starts_with('/') {
                file.clone()
            } else {
                format!("/{}", file)
            };

            if !is_library_path(&file_path) {
                continue;
            }

            if let Some(basename) = Path::new(&file_path).file_name().and_then(|n| n.to_str()) {
                // self libs have highest priority - overwrite any dep entries
                lookup.insert(
                    basename.to_string(),
                    (
                        self_provider_key.to_string(),
                        file_path,
                        self_files_commit.clone(),
                    ),
                );
            }
        }
    }

    Ok(lookup)
}

/// Find files commit for a package by looking up its manifest's checksum.
/// Format: {package.checksum}/files
fn find_or_create_files_commit(
    repo_path: &str,
    provider_key: &str,
    commits: &[String],
    verbose: bool,
) -> io::Result<String> {
    // parse provider key to get package info
    // e.g., "libs/system/glibc/2.39" -> namespace_path="libs/system", slug="glibc", version="2.39"
    let parts: Vec<&str> = provider_key.split('/').collect();
    if parts.len() < 2 {
        // can't parse, use first commit as fallback
        return Ok(commits.first().cloned().unwrap_or_default());
    }

    let version = parts.last().unwrap();
    let slug = parts.get(parts.len().saturating_sub(2)).unwrap();
    let namespace_path = parts[..parts.len().saturating_sub(2)].join("/");

    // try to find the manifest and determine the address hash
    // manifest path: pkg/{namespace_path}/{slug}.yaml
    let manifest_path_str = format!("pkg/{}/{}.yaml", namespace_path, slug);
    let manifest_path = PathBuf::from(&manifest_path_str);

    if let Ok(manifest_data) =
        load_manifest_from_source(&ManifestSource::Path(manifest_path.clone()))
    {
        if let crate::manifest::types::ManifestData::Package(dep_manifest) = manifest_data {
            // determine address hash: use checksum if stable, else use manifest git blob SHA
            let has_stable_checksum = dep_manifest.package.checksum.is_some()
                && dep_manifest.package.stable_checksum.unwrap_or(true);

            let address_hash = if has_stable_checksum {
                dep_manifest.package.checksum.clone().unwrap()
            } else {
                // fallback: use manifest's git blob SHA (input-addressed)
                hash_file_content(&manifest_path)?
            };

            // content-addressed: {hash}/files
            let files_ref = format!("{}/files", address_hash);
            let repo = OstreeRepo::open(repo_path)?;

            // check if it exists
            if repo.resolve_ref(&files_ref).is_ok() {
                if verbose {
                    println!("    Using files commit: {}", files_ref);
                }
                return Ok(files_ref);
            }

            // doesn't exist yet - create it
            let output_prefix = format!(
                "x86_64/pkg/{}/{}/{}/outputs/",
                namespace_path, slug, version
            );
            let output_refs = repo.refs(Some(&output_prefix))?;

            if !output_refs.is_empty() {
                if verbose {
                    println!(
                        "    Creating files commit for {} ({} outputs)",
                        provider_key,
                        output_refs.len()
                    );
                }
                return create_files_commit(
                    repo_path,
                    &output_refs,
                    &address_hash,
                    &namespace_path,
                    slug,
                    version,
                );
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Could not find manifest or outputs for {}", provider_key),
    ))
}

/// Extract provider key from commit ref.
/// e.g., "x86_64/pkg/libs/system/glibc/2.39/outputs/lib" -> "libs/system/glibc/2.39"
fn extract_provider_key(commit: &str) -> String {
    let parts: Vec<&str> = commit.split('/').collect();

    // find "pkg" index
    let pkg_idx = parts.iter().position(|&p| p == "pkg");

    // find "outputs" or "bundles" index
    let end_idx = parts
        .iter()
        .position(|&p| p == "outputs" || p == "bundles" || p == "files");

    match (pkg_idx, end_idx) {
        (Some(start), Some(end)) if end > start + 1 => parts[start + 1..end].join("/"),
        _ => commit.to_string(),
    }
}

/// Check if a path looks like a library file.
fn is_library_path(path: &str) -> bool {
    path.contains("/lib/")
        || path.contains("/lib64/")
        || path.ends_with(".so")
        || path.contains(".so.")
}

/// Create a `{hash}/files` commit containing the union of all outputs.
/// The hash is provided by the caller (either package.checksum or manifest git blob SHA).
/// Returns the OSTree ref for the files commit.
fn create_files_commit(
    repo_path: &str,
    output_refs: &[String],
    address_hash: &str,
    namespace_path: &str,
    slug: &str,
    version: &str,
) -> io::Result<String> {
    let repo = OstreeRepo::open(repo_path)?;

    // create temp dir for the union checkout
    let repo_tmp = PathBuf::from(repo_path).join("tmp");
    fs::create_dir_all(&repo_tmp)?;
    let temp = TempDir::new_in(&repo_tmp)?;
    let union_dir = temp.path().join("files");
    fs::create_dir_all(&union_dir)?;

    // checkout each output with --union
    for output_ref in output_refs {
        repo.checkout(output_ref, &union_dir, true)?;
    }

    // content-addressed: {hash}/files
    let files_ref = format!("{}/files", address_hash);

    // commit the union directory
    // keep package info in metadata for debugging/back-links
    let metadata = vec![
        ("nex.address_hash".to_string(), address_hash.to_string()),
        (
            "nex.package".to_string(),
            format!("{}/{}/{}", namespace_path, slug, version),
        ),
        (
            "nex.output_count".to_string(),
            output_refs.len().to_string(),
        ),
    ];

    commit_to_ostree(repo_path, &files_ref, &union_dir, &metadata)?;

    Ok(files_ref)
}

/// Scan an ELF file for DT_NEEDED libraries.
fn scan_elf_for_deps(path: &Path) -> io::Result<Vec<String>> {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };

    // check ELF magic
    if data.len() < 4 || &data[..4] != b"\x7FELF" {
        return Ok(Vec::new());
    }

    match Object::parse(&data) {
        Ok(Object::Elf(elf)) => {
            let mut result: Vec<String> = elf
                .libraries
                .iter()
                .map(|lib| lib.trim().to_string())
                .filter(|lib| !lib.is_empty())
                .collect();

            // include interpreter (ld-linux-x86-64.so.2)
            if let Some(interp) = elf.interpreter {
                let basename = Path::new(interp)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(interp)
                    .to_string();
                if !basename.is_empty() && !result.contains(&basename) {
                    result.push(basename);
                }
            }

            Ok(result)
        }
        _ => Ok(Vec::new()),
    }
}

/// Scan a checkout directory for all files.
fn scan_files_in_checkout(checkout_dir: &Path) -> io::Result<Vec<String>> {
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(checkout_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() && !entry.file_type().is_symlink() {
            continue;
        }

        if let Ok(rel_path) = entry.path().strip_prefix(checkout_dir) {
            let path_str = format!("/{}", rel_path.display());
            files.push(path_str);
        }
    }

    Ok(files)
}

/// Update the manifest file with computed dependencies.
/// Only updates the `needs` field for existing files, plus `resolution`.
/// Does NOT modify which files are in each output or the output structure.
fn update_manifest_file(
    manifest_path: &Path,
    new_outputs: &HashMap<String, Vec<FileEntry>>,
    resolution: &HashMap<String, String>,
) -> io::Result<()> {
    // read the original file
    let content = fs::read_to_string(manifest_path)?;

    // parse as YAML to preserve structure
    let mut doc: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // build a lookup map: path -> needs
    let mut needs_lookup: HashMap<String, Vec<String>> = HashMap::new();
    for file_entries in new_outputs.values() {
        for fe in file_entries {
            needs_lookup.insert(fe.path.clone(), fe.needs.clone());
        }
    }

    // update outputs section - only modify the `needs` field for existing files
    if let Some(outputs) = doc.get_mut("outputs") {
        if let serde_yaml::Value::Mapping(outputs_map) = outputs {
            for (_output_key, output_value) in outputs_map.iter_mut() {
                if let serde_yaml::Value::Mapping(output_map) = output_value {
                    if let Some(serde_yaml::Value::Sequence(files)) =
                        output_map.get_mut(&serde_yaml::Value::String("files".to_string()))
                    {
                        for file_entry in files.iter_mut() {
                            if let serde_yaml::Value::Mapping(file_map) = file_entry {
                                // get the path of this file
                                let path = file_map
                                    .get(&serde_yaml::Value::String("path".to_string()))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                if let Some(path) = path {
                                    // look up the computed needs for this file
                                    if let Some(needs) = needs_lookup.get(&path) {
                                        // remove old needs/deps
                                        file_map.remove(&serde_yaml::Value::String(
                                            "needs".to_string(),
                                        ));
                                        file_map
                                            .remove(&serde_yaml::Value::String("deps".to_string()));

                                        // add new needs if non-empty
                                        if !needs.is_empty() {
                                            let needs_seq: Vec<serde_yaml::Value> = needs
                                                .iter()
                                                .map(|n| serde_yaml::Value::String(n.clone()))
                                                .collect();
                                            file_map.insert(
                                                serde_yaml::Value::String("needs".to_string()),
                                                serde_yaml::Value::Sequence(needs_seq),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // add resolution at root level
    if !resolution.is_empty() {
        let resolution_map: serde_yaml::Mapping = resolution
            .iter()
            .map(|(k, v)| {
                (
                    serde_yaml::Value::String(k.clone()),
                    serde_yaml::Value::String(v.clone()),
                )
            })
            .collect();
        doc.as_mapping_mut().unwrap().insert(
            serde_yaml::Value::String("resolution".to_string()),
            serde_yaml::Value::Mapping(resolution_map),
        );
    }

    // remove providers if present (no longer used)
    doc.as_mapping_mut()
        .unwrap()
        .remove(&serde_yaml::Value::String("providers".to_string()));

    // write back with blank line before resolution section
    let yaml_content = serde_yaml::to_string(&doc)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let new_content = yaml_content.replace("\nresolution:\n", "\n\nresolution:\n");

    fs::write(manifest_path, new_content)?;

    Ok(())
}
