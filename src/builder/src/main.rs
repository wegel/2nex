use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::mem;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;
use num_cpus;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use walkdir::WalkDir;

use std::fmt;
use std::process;

pub mod runtime;

mod utils;

use runtime::scanner::{RuntimeScanResult, RuntimeScanner};
use utils::determine_category;

#[derive(Parser)]
#[clap(version = "1.0", author = "Your Name")]
struct Opts {
    #[clap(value_name = "REPO", help = "Path to the OSTree repository")]
    repo_path: String,
    #[clap(value_name = "MANIFEST", help = "Path to the manifest file")]
    manifest_file: String,
    #[clap(long, help = "Validate build reproducibility")]
    validate_reproducibility: bool,
    #[clap(
        long,
        help = "Run the build script on the host's filesystem (for bootstrapping)"
    )]
    bootstrap: bool,
    #[clap(long, help = "Skip runtime dependency scanning")]
    skip_runtime_deps: bool,
    #[clap(
        long,
        help = "Include per-reference explanations in runtime dependency output"
    )]
    runtime_deps_verbose: bool,
    #[clap(
        long,
        help = "Treat missing files during runtime dependency scanning as warnings instead of errors"
    )]
    allow_missing_runtime_files: bool,
    #[clap(
        long,
        help = "Rewrite outputs.*.requires based on the runtime dependency scanner (implies scanning)"
    )]
    update_outputs_requires: bool,
    #[clap(
        long,
        help = "Update outputs.*.requires using existing OSTree outputs without rebuilding"
    )]
    update_outputs_requires_only: bool,
    #[clap(
        long,
        help = "Rewrite OSTree output/bundle metadata without rebuilding (package manifests only)"
    )]
    refresh_ostree_metadata: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Manifest {
    package: Package,
    dependencies: Vec<Dependency>,
    sources: Vec<Source>,
    build: Build,
    #[serde(deserialize_with = "deserialize_outputs")]
    outputs: HashMap<String, OutputSpec>,
    #[serde(deserialize_with = "deserialize_bundles")]
    bundles: HashMap<String, Bundle>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Package {
    name: String,
    slug: String,
    version: String,
    flavor: String,
    checksum: Option<String>,
    stable_checksum: Option<bool>,
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ManifestKind {
    Package,
    System,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct SystemMeta {
    name: String,
    slug: String,
    version: String,
    #[serde(default)]
    architecture: Option<String>,
    #[serde(default)]
    boot_method: Option<String>,
    #[serde(default)]
    description: Option<String>,
    checksum: Option<String>,
    #[serde(default)]
    stable_checksum: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct SystemManifest {
    #[serde(default)]
    schema: Option<u32>,
    system: SystemMeta,
    #[serde(default)]
    packages: Vec<SystemPackage>,
    #[serde(default)]
    dependencies: Vec<Dependency>,
    #[serde(default)]
    sources: Vec<Source>,
    build: Build,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct SystemPackage {
    commit: String,
    #[serde(default)]
    name: Option<String>,
}

enum ManifestData {
    Package(Manifest),
    System(SystemManifest),
}

fn detect_manifest_kind(doc: &Value) -> ManifestKind {
    if let Some(kind) = doc.get("kind").and_then(|v| v.as_str()) {
        if kind.eq_ignore_ascii_case("system") {
            return ManifestKind::System;
        }
    }
    if doc.get("system").is_some() && doc.get("package").is_none() {
        ManifestKind::System
    } else {
        ManifestKind::Package
    }
}

fn validate_system_manifest(manifest: &SystemManifest) -> io::Result<()> {
    if manifest.packages.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "System manifests must specify at least one entry under 'packages'",
        ));
    }
    if manifest.system.version.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "System manifests must set system.version",
        ));
    }
    Ok(())
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Dependency {
    commit: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Source {
    name: String,
    url: Option<String>,
    file: Option<String>,
    sha256: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Build {
    script: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Bundle {
    #[serde(default)]
    includes: Vec<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    suggests: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BundleDef {
    Simple(Vec<String>),
    Detailed(Bundle),
}

impl From<BundleDef> for Bundle {
    fn from(def: BundleDef) -> Self {
        match def {
            BundleDef::Simple(includes) => Bundle {
                includes,
                requires: Vec::new(),
                suggests: Vec::new(),
            },
            BundleDef::Detailed(bundle) => bundle,
        }
    }
}

fn deserialize_bundles<'de, D>(deserializer: D) -> Result<HashMap<String, Bundle>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: HashMap<String, BundleDef> = HashMap::deserialize(deserializer)?;
    Ok(raw.into_iter().map(|(k, v)| (k, v.into())).collect())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct OutputSpec {
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    suggests: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OutputDef {
    Simple(Vec<String>),
    Detailed(OutputSpec),
}

impl From<OutputDef> for OutputSpec {
    fn from(def: OutputDef) -> Self {
        match def {
            OutputDef::Simple(files) => OutputSpec {
                files,
                requires: Vec::new(),
                suggests: Vec::new(),
            },
            OutputDef::Detailed(spec) => spec,
        }
    }
}

fn deserialize_outputs<'de, D>(deserializer: D) -> Result<HashMap<String, OutputSpec>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: HashMap<String, OutputDef> = HashMap::deserialize(deserializer)?;
    Ok(raw.into_iter().map(|(k, v)| (k, v.into())).collect())
}

fn main() -> io::Result<()> {
    let opts: Opts = Opts::parse();
    if opts.refresh_ostree_metadata
        && (opts.update_outputs_requires
            || opts.update_outputs_requires_only
            || opts.validate_reproducibility)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--refresh-ostree-metadata cannot be combined with build/update flags",
        ));
    }
    match load_manifest(&opts.manifest_file)? {
        ManifestData::Package(mut manifest) => {
            if opts.refresh_ostree_metadata {
                refresh_package_metadata(&opts.repo_path, &manifest)?;
                Ok(())
            } else {
                build_package_manifest(&opts, &mut manifest)
            }
        }
        ManifestData::System(manifest) => {
            if opts.refresh_ostree_metadata {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--refresh-ostree-metadata only applies to package manifests",
                ));
            }
            build_system_manifest(&opts, &manifest)
        }
    }
}

fn build_package_manifest(opts: &Opts, manifest: &mut Manifest) -> io::Result<()> {
    let base_dir = "./build_rootfs";
    let download_dir = "./inputs_cache";
    fs::create_dir_all(download_dir)?;

    let dependency_commits = resolve_dependency_closure(&manifest.dependencies, &opts.repo_path)?;
    let wants_update_outputs = opts.update_outputs_requires || opts.update_outputs_requires_only;
    let runtime_scanner = RuntimeScanner::new(&opts.repo_path, &dependency_commits)
        .with_allow_missing_files(opts.allow_missing_runtime_files);

    if opts.update_outputs_requires_only {
        stage_existing_outputs(manifest, base_dir, &opts.repo_path)?;
        let runtime_result = runtime_scanner.scan(
            &manifest.package.name,
            &manifest.package.version,
            base_dir,
            opts.runtime_deps_verbose,
        )?;
        update_manifest_outputs(&opts.manifest_file, &runtime_result)?;
        apply_runtime_requires(manifest, &runtime_result);
        println!(
            "Updated outputs.requires for {} without rebuilding",
            opts.manifest_file
        );
        return Ok(());
    }

    setup_composite_rootfs(base_dir, &opts.repo_path, &dependency_commits)?;
    let input_env_vars = handle_inputs(&manifest.sources, download_dir, base_dir, opts.bootstrap)?;

    let package_name = &manifest.package.name;
    let package_version = &manifest.package.version;
    let package_flavor = &manifest.package.flavor;

    println!(
        "Building {} {} for flavor {}",
        package_name, package_version, package_flavor
    );

    let mut env_vars = HashMap::new();
    env_vars.extend(input_env_vars);
    let build_script = manifest.build.script.clone();

    run_build_script(&build_script, base_dir, &env_vars, opts.bootstrap)?;

    let need_runtime_scan = !opts.skip_runtime_deps || wants_update_outputs;
    let runtime_analysis: Option<RuntimeScanResult> = if need_runtime_scan {
        Some(runtime_scanner.scan(
            &manifest.package.name,
            &manifest.package.version,
            base_dir,
            opts.runtime_deps_verbose,
        )?)
    } else {
        None
    };

    if wants_update_outputs {
        if let Some(result) = runtime_analysis.as_ref() {
            update_manifest_outputs(&opts.manifest_file, result)?;
            apply_runtime_requires(manifest, result);
        } else {
            println!("Skipping output requires update because runtime scanning was disabled.");
        }
    }

    let runtime_suggestions: Option<&RuntimeScanResult> = if opts.skip_runtime_deps {
        None
    } else {
        runtime_analysis.as_ref()
    };

    verify_and_commit_outputs(
        manifest,
        base_dir,
        &opts.repo_path,
        runtime_suggestions,
        opts.runtime_deps_verbose,
    )?;

    create_and_commit_bundles(manifest, base_dir, &opts.repo_path)?;

    println!("Build, packaging, and commit to OSTree completed for all outputs.");

    let output_dir = Path::new(base_dir).join("2nex/out");
    let checksum = calculate_output_checksum(&output_dir)?;
    println!("Build output checksum: {}", checksum);

    if let Some(expected_checksum) = &manifest.package.checksum {
        if checksum != *expected_checksum {
            eprintln!(
                "Checksum mismatch. Expected: {}, Calculated: {}",
                expected_checksum, checksum
            );
            process::exit(-2);
        } else {
            println!("Checksum verified successfully.");
        }
    }

    if opts.validate_reproducibility {
        println!("Validating build reproducibility by building the package a second time.");
        fs::remove_dir_all(base_dir)?;
        setup_composite_rootfs(base_dir, &opts.repo_path, &dependency_commits)?;
        handle_inputs(&manifest.sources, download_dir, base_dir, opts.bootstrap)?;
        run_build_script(&build_script, base_dir, &env_vars, opts.bootstrap)?;
        verify_and_commit_outputs(
            manifest,
            base_dir,
            &opts.repo_path,
            runtime_suggestions,
            opts.runtime_deps_verbose,
        )?;
        create_and_commit_bundles(manifest, base_dir, &opts.repo_path)?;

        let second_checksum = calculate_output_checksum(&output_dir)?;
        println!("Second build output checksum: {}", second_checksum);

        if checksum == second_checksum {
            println!("Build is reproducible. Checksums match.");
        } else {
            println!("Build is not reproducible. Checksums do not match.");
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Build is not reproducible.",
            ));
        }
    }

    append_checksum_file(&manifest.package, &checksum, &Path::new("checksums.txt"))?;

    Ok(())
}

fn refresh_package_metadata(repo_path: &str, manifest: &Manifest) -> io::Result<()> {
    println!(
        "Refreshing OSTree metadata for {}/{} ({})",
        manifest.package.slug, manifest.package.version, manifest.package.flavor
    );
    refresh_output_branches(repo_path, manifest)?;
    refresh_bundle_branches(repo_path, manifest)?;
    println!("Finished refreshing metadata for {}", manifest.package.slug);
    Ok(())
}

fn refresh_output_branches(repo_path: &str, manifest: &Manifest) -> io::Result<()> {
    for (category, spec) in &manifest.outputs {
        if category == "discard" {
            continue;
        }
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, category
        );
        ensure_branch_exists(repo_path, &branch_name)?;
        let metadata = output_branch_metadata(manifest, spec)?;
        rewrite_branch_metadata(repo_path, &branch_name, &metadata)?;
    }
    Ok(())
}

fn refresh_bundle_branches(repo_path: &str, manifest: &Manifest) -> io::Result<()> {
    for (bundle_name, bundle) in &manifest.bundles {
        let branch_name = format!(
            "x86_64/{}/{}/{}/bundles/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, bundle_name
        );
        ensure_branch_exists(repo_path, &branch_name)?;
        let metadata = bundle_branch_metadata(manifest, bundle)?;
        rewrite_branch_metadata(repo_path, &branch_name, &metadata)?;
    }
    Ok(())
}

fn ensure_branch_exists(repo_path: &str, branch: &str) -> io::Result<()> {
    let mut command = Command::new("unshare");
    command.args(&["--user", "--map-root-user", "--"]);
    command.arg("ostree");
    command.arg("rev-parse");
    command.arg("--repo");
    command.arg(repo_path);
    command.arg(branch);
    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Branch {} missing in {}: {}",
                branch,
                repo_path,
                String::from_utf8_lossy(&output.stderr)
            ),
        ))
    }
}

fn rewrite_branch_metadata(
    repo_path: &str,
    branch: &str,
    metadata: &[(String, String)],
) -> io::Result<()> {
    println!("Rewriting metadata for {}", branch);
    let mut command = Command::new("unshare");
    command.args(&["--map-root-user", "--user", "--"]);
    command.arg("ostree");
    command.arg("commit");
    command.arg("--repo").arg(repo_path);
    command.arg("--branch").arg(branch);
    command.arg(format!("--tree=ref={}", branch));
    command.arg("--no-xattrs");
    command.arg("--no-bindings");

    for (key, value) in metadata {
        let metadata_arg = format!("{}={}", key, value);
        command.arg("--add-metadata-string");
        command.arg(metadata_arg);
    }

    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Failed to rewrite metadata for {}: {}",
                branch,
                String::from_utf8_lossy(&output.stderr)
            ),
        ))
    }
}

fn build_system_manifest(opts: &Opts, manifest: &SystemManifest) -> io::Result<()> {
    if opts.update_outputs_requires || opts.update_outputs_requires_only {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "System manifests do not define outputs, so --update-outputs-requires flags are invalid.",
        ));
    }
    if opts.runtime_deps_verbose || opts.skip_runtime_deps {
        println!("Note: runtime dependency scanning is not available for system manifests yet.");
    }

    let base_dir = "./build_rootfs";
    let download_dir = "./inputs_cache";
    fs::create_dir_all(download_dir)?;

    let dependency_commits = resolve_dependency_closure(&manifest.dependencies, &opts.repo_path)?;
    let package_dependency_specs = dependencies_from_system_packages(&manifest.packages);
    let package_commits = resolve_dependency_closure(&package_dependency_specs, &opts.repo_path)?;

    setup_composite_rootfs(base_dir, &opts.repo_path, &dependency_commits)?;
    layer_commits_into_rootfs(base_dir, &opts.repo_path, &package_commits)?;
    materialize_system_packages(base_dir, &opts.repo_path, &package_commits)?;

    let env_vars = build_system_env_vars(manifest, download_dir, base_dir, opts.bootstrap)?;

    println!(
        "Building system {} {}",
        manifest.system.slug, manifest.system.version
    );

    run_build_script(&manifest.build.script, base_dir, &env_vars, opts.bootstrap)?;

    let target_dir = Path::new(base_dir).join("target");
    let checksum = calculate_output_checksum(&target_dir)?;
    println!("System build checksum: {}", checksum);

    if let Some(expected_checksum) = &manifest.system.checksum {
        if checksum != *expected_checksum {
            eprintln!(
                "Checksum mismatch. Expected: {}, Calculated: {}",
                expected_checksum, checksum
            );
            process::exit(-2);
        } else {
            println!("Checksum verified successfully.");
        }
    }

    commit_system_rootfs(
        manifest,
        base_dir,
        &opts.repo_path,
        &package_commits,
        &dependency_commits,
        &checksum,
    )?;

    println!(
        "System commit stored at systems/{}/{}",
        manifest.system.slug, manifest.system.version
    );

    if opts.validate_reproducibility {
        println!("Validating build reproducibility by building the system a second time.");
        fs::remove_dir_all(base_dir)?;

        setup_composite_rootfs(base_dir, &opts.repo_path, &dependency_commits)?;
        layer_commits_into_rootfs(base_dir, &opts.repo_path, &package_commits)?;
        materialize_system_packages(base_dir, &opts.repo_path, &package_commits)?;

        let env_vars = build_system_env_vars(manifest, download_dir, base_dir, opts.bootstrap)?;
        run_build_script(&manifest.build.script, base_dir, &env_vars, opts.bootstrap)?;

        let second_checksum = calculate_output_checksum(&target_dir)?;
        println!("Second build checksum: {}", second_checksum);

        if checksum == second_checksum {
            println!("Build is reproducible. Checksums match.");
        } else {
            println!("Build is not reproducible. Checksums do not match.");
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Build is not reproducible.",
            ));
        }

        commit_system_rootfs(
            manifest,
            base_dir,
            &opts.repo_path,
            &package_commits,
            &dependency_commits,
            &second_checksum,
        )?;
    }

    Ok(())
}

fn build_system_env_vars(
    manifest: &SystemManifest,
    download_dir: &str,
    base_dir: &str,
    bootstrap: bool,
) -> io::Result<HashMap<String, String>> {
    let mut env_vars = handle_inputs(&manifest.sources, download_dir, base_dir, bootstrap)?;
    env_vars.insert("SYSTEM_NAME".to_string(), manifest.system.name.clone());
    env_vars.insert("SYSTEM_SLUG".to_string(), manifest.system.slug.clone());
    env_vars.insert(
        "SYSTEM_VERSION".to_string(),
        manifest.system.version.clone(),
    );
    env_vars.insert("TARGET_DIR".to_string(), "/target".to_string());
    env_vars.insert("SYSTEM_TARGET".to_string(), "/target".to_string());
    if let Some(arch) = &manifest.system.architecture {
        env_vars.insert("SYSTEM_ARCH".to_string(), arch.clone());
    }
    if let Some(boot) = &manifest.system.boot_method {
        env_vars.insert("SYSTEM_BOOT_METHOD".to_string(), boot.clone());
    }
    if let Some(desc) = &manifest.system.description {
        env_vars.insert("SYSTEM_DESCRIPTION".to_string(), desc.clone());
    }
    Ok(env_vars)
}

fn dependencies_from_system_packages(packages: &[SystemPackage]) -> Vec<Dependency> {
    packages
        .iter()
        .map(|pkg| Dependency {
            commit: pkg.commit.clone(),
            name: pkg.name.clone(),
        })
        .collect()
}

fn materialize_system_packages(
    base_dir: &str,
    repo_path: &str,
    package_commits: &[String],
) -> io::Result<()> {
    let target_dir = Path::new(base_dir).join("target");
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
    }
    fs::create_dir_all(&target_dir)?;

    for commit in package_commits {
        checkout_ostree_into(repo_path, commit, &target_dir, true)?;
    }

    Ok(())
}

fn commit_system_rootfs(
    manifest: &SystemManifest,
    base_dir: &str,
    repo_path: &str,
    package_commits: &[String],
    dependency_commits: &[String],
    checksum: &str,
) -> io::Result<()> {
    let target_dir = Path::new(base_dir).join("target");
    if !target_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "System target directory missing after build",
        ));
    }

    let branch_name = format!(
        "systems/{}/{}",
        manifest.system.slug, manifest.system.version
    );
    let mut metadata = Vec::new();
    metadata.push(("nex.system.name".to_string(), manifest.system.name.clone()));
    metadata.push(("nex.system.slug".to_string(), manifest.system.slug.clone()));
    metadata.push((
        "nex.system.version".to_string(),
        manifest.system.version.clone(),
    ));
    metadata.push(("nex.build.checksum".to_string(), checksum.to_string()));
    if let Some(desc) = &manifest.system.description {
        metadata.push(("nex.system.description".to_string(), desc.clone()));
    }
    if let Some(arch) = &manifest.system.architecture {
        metadata.push(("nex.system.arch".to_string(), arch.clone()));
    }
    if let Some(boot) = &manifest.system.boot_method {
        metadata.push(("nex.system.boot_method".to_string(), boot.clone()));
    }
    if let Some(encoded) = encode_metadata_list(package_commits)? {
        metadata.push(("nex.system.packages".to_string(), encoded));
    }
    if let Some(encoded) = encode_metadata_list(dependency_commits)? {
        metadata.push(("nex.system.dependencies".to_string(), encoded));
    }

    commit_to_ostree(&target_dir, &branch_name, repo_path, &metadata)
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.flavor, self.slug)
    }
}

fn append_checksum_file(package: &Package, checksum: &str, file_path: &Path) -> io::Result<()> {
    // Step 1: Calculate the maximum width of the first column
    let mut max_first_column_width = 0;
    if file_path.exists() {
        let file = File::open(file_path)?;
        for line in io::BufReader::new(file).lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 1 {
                max_first_column_width = max_first_column_width.max(parts[0].len());
            }
        }
    }

    // Step 2: Append the new entry to the file
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;
    if max_first_column_width > 0 {
        // If the file was not empty, add a newline before appending
        writeln!(file)?;
    }
    let padded_first_column = format!(
        "{:<width$}",
        package.to_string(),
        width = max_first_column_width + 3 // Adjust for padding
    );
    writeln!(file, "{} {}", padded_first_column, checksum)?;

    Ok(())
}

fn load_manifest(file_path: &str) -> io::Result<ManifestData> {
    let manifest_str = fs::read_to_string(file_path)?;
    let doc: Value = serde_yaml::from_str(&manifest_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    match detect_manifest_kind(&doc) {
        ManifestKind::Package => {
            let manifest: Manifest = serde_yaml::from_value(doc)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Ok(ManifestData::Package(manifest))
        }
        ManifestKind::System => {
            let sys: SystemManifest = serde_yaml::from_value(doc)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            validate_system_manifest(&sys)?;
            Ok(ManifestData::System(sys))
        }
    }
}

fn resolve_dependency_closure(
    dependencies: &[Dependency],
    repo_path: &str,
) -> io::Result<Vec<String>> {
    resolve_dependency_closure_with_fetch(dependencies, |commit| {
        fetch_requires_from_repo(repo_path, commit)
    })
}

fn resolve_dependency_closure_with_fetch<F>(
    dependencies: &[Dependency],
    mut fetch: F,
) -> io::Result<Vec<String>>
where
    F: FnMut(&str) -> io::Result<Vec<String>>,
{
    let mut resolved = Vec::new();
    let mut seen = HashSet::new();
    let mut visiting = HashSet::new();
    let mut stack = Vec::new();

    for dep in dependencies {
        visit_commit(
            &dep.commit,
            &mut fetch,
            &mut seen,
            &mut visiting,
            &mut stack,
            &mut resolved,
        )?;
    }

    Ok(resolved)
}

fn visit_commit<F>(
    commit: &str,
    fetch: &mut F,
    seen: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
    stack: &mut Vec<String>,
    resolved: &mut Vec<String>,
) -> io::Result<()>
where
    F: FnMut(&str) -> io::Result<Vec<String>>,
{
    if seen.contains(commit) {
        return Ok(());
    }
    if !visiting.insert(commit.to_string()) {
        let cycle = build_cycle_path(stack, commit);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Runtime dependency cycle detected: {}", cycle),
        ));
    }

    stack.push(commit.to_string());
    let requires = fetch(commit)?;
    for req in requires {
        visit_commit(&req, fetch, seen, visiting, stack, resolved)?;
    }
    stack.pop();
    visiting.remove(commit);
    seen.insert(commit.to_string());
    resolved.push(commit.to_string());
    Ok(())
}

fn build_cycle_path(stack: &[String], repeat: &str) -> String {
    if let Some(pos) = stack.iter().position(|c| c == repeat) {
        let mut path = stack[pos..].join(" -> ");
        path.push_str(" -> ");
        path.push_str(repeat);
        path
    } else if stack.is_empty() {
        repeat.to_string()
    } else {
        stack.join(" -> ")
    }
}

fn fetch_requires_from_repo(repo_path: &str, commit: &str) -> io::Result<Vec<String>> {
    let mut combined = Vec::new();
    for key in ["nex.bundle.requires", "nex.output.requires"] {
        let mut entries = read_metadata_list(repo_path, commit, key)?;
        combined.append(&mut entries);
    }
    combined.retain(|entry| !entry.is_empty());
    Ok(combined)
}

fn stage_existing_outputs(manifest: &Manifest, base_dir: &str, repo_path: &str) -> io::Result<()> {
    let base_path = Path::new(base_dir);
    if base_path.exists() {
        fs::remove_dir_all(base_path)?;
    }
    let out_dir = base_path.join("2nex/out");
    fs::create_dir_all(&out_dir)?;

    for category in manifest.outputs.keys() {
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, category
        );
        println!(
            "Checking out existing outputs from {} into {}",
            branch_name,
            out_dir.display()
        );
        checkout_ostree_into(repo_path, &branch_name, &out_dir, true)?;
    }

    Ok(())
}

fn setup_composite_rootfs(
    base_dir: &str,
    repo_path: &str,
    dependency_commits: &[String],
) -> io::Result<()> {
    println!("Setting up composite rootfs at {}", base_dir);
    fs::create_dir_all(base_dir)?;

    let work_dir = Path::new(base_dir).join("2nex/work");
    let out_dir = Path::new(base_dir).join("2nex/out");
    let tmp_dir = Path::new(base_dir).join("tmp");

    for dir in &[&work_dir, &out_dir, &tmp_dir] {
        println!("Creating directory: {}", dir.display());
        if dir.exists() {
            println!("Removing existing {}", dir.display());
            fs::remove_dir_all(dir)?;
        }
        fs::create_dir_all(dir)?;
    }

    for commit in dependency_commits {
        checkout_ostree_into(repo_path, commit, Path::new(base_dir), true)?;
    }

    Ok(())
}

fn layer_commits_into_rootfs(
    base_dir: &str,
    repo_path: &str,
    commits: &[String],
) -> io::Result<()> {
    for commit in commits {
        checkout_ostree_into(repo_path, commit, Path::new(base_dir), true)?;
    }
    Ok(())
}

fn handle_inputs(
    sources: &[Source],
    download_dir: &str,
    build_dir: &str,
    is_bootstrap: bool,
) -> io::Result<HashMap<String, String>> {
    println!("Handling inputs");

    let mut input_env_vars = HashMap::new();
    let current_dir = env::current_dir().expect("Failed to get current directory");

    for (i, source) in sources.iter().enumerate() {
        let input = fetch_and_verify_input(source, download_dir)?;
        let inputs_dir = Path::new(build_dir).join("inputs");
        fs::create_dir_all(&inputs_dir)?;
        let input_path = inputs_dir.join(input.file_name().unwrap());
        fs::copy(input.clone(), &input_path)?;

        let path_str = if is_bootstrap {
            current_dir.join(&input_path).to_str().unwrap().to_string()
        } else {
            let relative_path = Path::new("./inputs").join(input_path.file_name().unwrap());
            relative_path.to_str().unwrap().to_string()
        };

        input_env_vars.insert(format!("SOURCE{}", i), path_str.clone());
        let var_name = format!("SOURCE_{}", source.name);
        input_env_vars.insert(var_name, path_str);
    }

    Ok(input_env_vars)
}

fn run_build_script(
    build_script: &str,
    build_dir: &str,
    env_vars: &HashMap<String, String>,
    bootstrap: bool,
) -> io::Result<()> {
    println!("Running build script in an isolated environment using unshare");

    let mut env = HashMap::new();
    env.insert("HOME".to_string(), "/homeless/deterministic".to_string());
    env.insert("LC_ALL".to_string(), "C".to_string());
    env.insert("TZ".to_string(), "UTC".to_string());
    env.insert("LANG".to_string(), "en_US.UTF-8".to_string());
    env.insert("SOURCE_DATE_EPOCH".to_string(), "1704067200".to_string());
    env.insert(
        "GLIBC_TUNABLES".to_string(),
        "glibc.cpu.hwcaps=-RNDRAND".to_string(),
    );
    let num_cpus = num_cpus::get();
    env.insert("MAKEFLAGS".to_string(), format!("-j{num_cpus}"));

    let current_dir = env::current_dir().expect("Failed to get current directory");
    let build_dir_path = Path::new(build_dir);
    let build_dir_abs = if build_dir_path.is_absolute() {
        build_dir_path.to_path_buf()
    } else {
        current_dir.join(build_dir_path)
    };
    let build_dir_str = build_dir_abs
        .to_str()
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "build_dir must be valid UTF-8")
        })?
        .to_string();
    let tmpdir_path = build_dir_abs.join("2nex").join("tmp");
    std::fs::create_dir_all(&tmpdir_path)?;

    let unshare_command = vec![
        "unshare",
        "--user",
        "--pid",
        "--mount",
        "--uts",
        "--fork",
        "--ipc",
        "--net",
        "--map-root-user",
        "/usr/bin/bash",
        "-o",
        "errexit",
        "-o",
        "nounset",
        "-c",
    ];

    let mut command_args = unshare_command.clone();
    let launch_script = if bootstrap {
        let bootstrap_sysroot_path = build_dir_abs.join("bootstrap");
        let bootstrap_tools_path = bootstrap_sysroot_path.join("tools");
        let bootstrap_tools_path_display = bootstrap_tools_path.display();
        let workdir_path = build_dir_abs.join("2nex").join("work");
        let outdir_path = build_dir_abs.join("2nex").join("out");

        println!("Bootstrap mode.");

        // Unset all environment variables
        for (key, _) in std::env::vars() {
            std::env::remove_var(key);
        }

        env.insert(
            "PATH".to_string(),
            format!("{bootstrap_tools_path_display}/bin:/usr/bin").to_string(),
        );
        env.insert("TARGET".to_string(), "x86_64-2nex-linux-gnu".to_string());
        env.insert(
            "WORK_DIR".to_string(),
            workdir_path.to_str().unwrap().to_string(),
        );
        env.insert(
            "OUT_DIR".to_string(),
            outdir_path.to_str().unwrap().to_string(),
        );
        env.insert(
            "BOOTSTRAP_TOOLS".to_string(),
            bootstrap_tools_path.to_str().unwrap().to_string(),
        );
        env.insert(
            "BOOTSTRAP_SYSROOT".to_string(),
            bootstrap_sysroot_path.to_str().unwrap().to_string(),
        );
        env.insert(
            "CFLAGS".to_string(),
            "-march=x86-64 -mtune=generic -O2 -frandom-seed=424242".to_string(),
        );
        env.insert(
            "CXXFLAGS".to_string(),
            "-march=x86-64 -mtune=generic -O2 -frandom-seed=424242".to_string(),
        );
        env.insert(
            "LDFLAGS".to_string(),
            "-L/bootstrap/usr/lib -Wl,-O1,--sort-common,--as-needed,-z,now".to_string(),
        );

        format!(
            r#"
            {build_script}"#
        )
    } else {
        env.insert(
            "PATH".to_string(),
            "/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
        );
        env.insert("RUSTC_BOOTSTRAP".to_string(), "1".to_string());
        env.insert("CFLAGS".to_string(), "-march=x86-64 -mtune=generic -O2 -pipe -fno-plt -fexceptions -Wp,-D_FORTIFY_SOURCE=2 -Wformat -Werror=format-security -fstack-clash-protection -fcf-protection -fPIC -fno-common -fno-omit-frame-pointer -frandom-seed=424242".to_string());
        env.insert("CXXFLAGS".to_string(), "-march=x86-64 -mtune=generic -O2 -pipe -fno-plt -fexceptions -Wp,-D_FORTIFY_SOURCE=2 -Wformat -Werror=format-security -fstack-clash-protection -fcf-protection -Wp,-D_GLIBCXX_ASSERTIONS -fPIC -fno-common -fno-omit-frame-pointer -frandom-seed=424242".to_string());
        env.insert(
            "LDFLAGS".to_string(),
            "-Wl,-O1,--sort-common,--as-needed,-z,relro,-z,now".to_string(),
        );
        env.insert("LTOFLAGS".to_string(), "-flto=auto".to_string());
        env.insert("RUSTFLAGS".to_string(), "-C codegen-units=1 -C embed-bitcode=yes -C debuginfo=0 -C link-args=-fuse-ld=lld -C target-feature=+crt-static -C link-args=-frandom-seed=424242".to_string());
        env.insert("DEBUG_CFLAGS".to_string(), "-g".to_string());
        env.insert("DEBUG_CXXFLAGS".to_string(), "-g".to_string());
        env.insert("DEBUG_RUSTFLAGS".to_string(), "-C debuginfo=2".to_string());
        env.insert("TARGET".to_string(), "x86_64-pc-linux-gnu".to_string());
        env.insert("WORK_DIR".to_string(), "/2nex/work".to_string());
        env.insert("OUT_DIR".to_string(), "/2nex/out".to_string());

        let temp_file_path = tmpdir_path.join("build_script.sh");
        let mut temp_file = std::fs::File::create(&temp_file_path)?;

        // Write the build_script content to the temporary file
        temp_file.write_all(b"#!/usr/bin/bash -eu\n")?;
        temp_file.write_all(build_script.as_bytes())?;

        format!(
            r#"
            if ! read -r current_hostname < /proc/sys/kernel/hostname; then
                echo 'Warning: Could not read current hostname; forcing to 2nex-builder' >&2
                current_hostname=""
            fi

            if [ "$current_hostname" != "2nex-builder" ]; then
                # Can't write /proc/sys/kernel/hostname inside a user namespace,
                # so set it here via the host's hostname binary before chrooting.
                echo 'Setting hostname to 2nex-builder'
                if ! hostname 2nex-builder; then
                    echo 'Warning: Failed to run hostname command' >&2
                fi
            fi

            mkdir -p {build_dir}/dev
            for D in null zero random urandom tty console full; do
                touch {build_dir}/dev/$D
                mount --bind /dev/$D {build_dir}/dev/$D
            done

            mkdir -p {build_dir}/dev/pts
            mount -t devpts devpts {build_dir}/dev/pts
            ln -sf /dev/pts/ptmx {build_dir}/dev/ptmx

            if [ -e {build_dir}/bin ]; then
                rmdir {build_dir}/lib
            fi
            
            if [ -e {build_dir}/lib64 ]; then
                rmdir {build_dir}/lib64
            fi
            
            if [ -e {build_dir}/sbin ]; then
                rmdir {build_dir}/usr/sbin
            fi
            
            if [ -e {build_dir}/lib64 ]; then
                rmdir {build_dir}/usr/lib64
            fi

            mkdir -p {build_dir}/usr

            if [ ! -e {build_dir}/bin ]; then
                ln -sf /usr/bin {build_dir}/bin
            fi

            if [ ! -e {build_dir}/lib ]; then
                ln -sf /usr/lib {build_dir}/lib
            fi

            if [ ! -e {build_dir}/sbin ]; then
                ln -sf /usr/bin {build_dir}/sbin
            fi

            if [ ! -e {build_dir}/lib64 ]; then
                ln -sf /usr/lib {build_dir}/lib64
            fi

            if [ ! -e {build_dir}/usr/lib64 ]; then
                ln -sf lib {build_dir}/usr/lib64
            fi

            if [ ! -e {build_dir}/usr/sbin ]; then
                ln -sf bin {build_dir}/usr/sbin
            fi

            chmod +x {build_dir}/2nex/tmp/build_script.sh
            unshare --root={build_dir} /2nex/tmp/build_script.sh
            "#,
            build_dir = build_dir_str
        )
    };
    command_args.push(&launch_script);

    for (key, value) in env_vars {
        env.insert(key.to_string(), value.to_string());
    }

    let mut command = Command::new("unshare");
    command.args(&command_args).envs(&env);

    println!(
        "Executing build script under unshare with env vars: {:?}",
        env
    );
    let mut child = command.spawn()?;

    // use the result of wait() to determine if the build script succeeded or not
    let result = child.wait()?;
    if result.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "Build script failed"))
    }
}

/// Verifies and commits outputs to OSTree branches based on the manifest.
///
/// This function takes the manifest, base directory, and repository path as input.
/// It verifies and commits the outputs specified in the manifest to the corresponding
/// OSTree branches. The function categorizes the output files, checks their existence,
/// moves them to the appropriate output directories, and commits them to the OSTree
/// repository.
fn verify_and_commit_outputs(
    manifest: &Manifest,
    base_dir: &str,
    repo_path: &str,
    runtime_suggestions: Option<&RuntimeScanResult>,
    verbose_reasons: bool,
) -> io::Result<()> {
    println!("Verifying and committing outputs to OSTree branches");

    let output_specs = &manifest.outputs;
    let out_dir = Path::new(base_dir).join("2nex/out");

    let all_out_files: Vec<String> = WalkDir::new(&out_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() || e.file_type().is_symlink())
        .map(|e| {
            e.path()
                .strip_prefix(&out_dir)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();

    let mut accounted_files = Vec::new();

    let outputs = categorize_files(&out_dir);
    println!("Suggested manifest outputs:");
    print_outputs(&outputs, runtime_suggestions, verbose_reasons);

    for (output_type, spec) in output_specs {
        if output_type == "discard" {
            continue;
        }

        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, output_type
        );

        for file_path in &spec.files {
            let source_path = out_dir.join(file_path.trim_start_matches('/'));
            if !source_path.is_symlink() && !source_path.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "File {} listed in manifest outputs does not exist.",
                        source_path.display()
                    ),
                ));
            }

            let target_dir_structure = source_path.parent().unwrap();
            let relative_parent = target_dir_structure.strip_prefix(&out_dir).unwrap();
            let output_dir = if relative_parent.as_os_str().is_empty() {
                // file is directly in out_dir (e.g., /init)
                out_dir.join(output_type)
            } else {
                out_dir.join(output_type).join(relative_parent)
            };

            // if output_dir path exists as a file (not dir), temporarily move it
            let temp_path = if output_dir.exists() && !output_dir.is_dir() {
                let tmp = output_dir.with_extension("tmp_rename");
                fs::rename(&output_dir, &tmp)?;
                Some(tmp)
            } else {
                None
            };

            fs::create_dir_all(&output_dir)?;

            // if we temporarily moved a file, move it back to its final location
            if let Some(tmp) = temp_path {
                fs::rename(&tmp, output_dir.join(source_path.file_name().unwrap()))?;
            } else {
                fs::rename(
                    &source_path,
                    output_dir.join(source_path.file_name().unwrap()),
                )?;
            }

            accounted_files.push(file_path.trim_start_matches('/').to_string());
        }

        let commit_output_dir = out_dir.join(output_type);

        let metadata = output_branch_metadata(manifest, spec)?;
        commit_to_ostree(&commit_output_dir, &branch_name, repo_path, &metadata)?;
    }

    let unaccounted_files: Vec<String> = all_out_files
        .into_iter()
        .filter(|f| !accounted_files.contains(f))
        .collect();
    if !unaccounted_files.is_empty() {
        println!(
            "The following files in /2nex/out are not accounted for in the manifest outputs: {:?}",
            unaccounted_files
        );
    }

    Ok(())
}

fn create_and_commit_bundles(
    manifest: &Manifest,
    _base_dir: &str,
    repo_path: &str,
) -> io::Result<()> {
    println!("Processing bundles");

    let bundles = &manifest.bundles;

    for (bundle_name, bundle) in bundles {
        commit_bundle(repo_path, bundle_name, bundle, manifest)?;
    }

    Ok(())
}

fn checkout_ostree_into(
    repo_path: &str,
    commit_id: &str,
    target_dir: &Path,
    union: bool,
) -> io::Result<()> {
    println!(
        "Checking out OSTree commit {} into {} (union: {})",
        commit_id,
        target_dir.display(),
        union
    );

    let mut command = Command::new("unshare");
    command.args(&["--user", "--map-root-user", "--"]);
    command.arg("ostree");
    command.arg("checkout");
    command.arg("--repo");
    command.arg(repo_path);
    if union {
        command.arg("--union");
    }
    command.arg(commit_id);
    command.arg(target_dir.to_str().unwrap());

    let output = command.output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Failed to checkout OSTree commit: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    Ok(())
}

fn commit_to_ostree(
    output_dir: &Path,
    branch_name: &str,
    repo_path: &str,
    metadata: &[(String, String)],
) -> io::Result<()> {
    println!(
        "Committing {} to OSTree branch {}",
        output_dir.display(),
        branch_name
    );

    let mut command = Command::new("unshare");
    command.args(&["--map-root-user", "--user", "--"]);

    command.arg("ostree");
    command.arg("commit");
    command.arg("--repo").arg(repo_path);
    command.arg("--branch").arg(branch_name);
    command.arg("--no-xattrs");
    command.arg("--no-bindings");

    for (key, value) in metadata {
        let metadata_arg = format!("{}={}", key, value);
        command.arg("--add-metadata-string");
        command.arg(metadata_arg);
    }

    command.arg(output_dir.to_str().unwrap());

    let output = command.output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Failed to commit to OSTree: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    Ok(())
}

fn encode_metadata_list(values: &[String]) -> io::Result<Option<String>> {
    if values.is_empty() {
        Ok(None)
    } else {
        serde_json::to_string(values)
            .map(Some)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

fn output_branch_metadata(
    manifest: &Manifest,
    spec: &OutputSpec,
) -> io::Result<Vec<(String, String)>> {
    let mut metadata = Vec::new();
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

fn bundle_branch_metadata(
    manifest: &Manifest,
    bundle: &Bundle,
) -> io::Result<Vec<(String, String)>> {
    let mut metadata = Vec::new();
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

fn read_metadata_list(repo_path: &str, commit: &str, key: &str) -> io::Result<Vec<String>> {
    let mut command = Command::new("unshare");
    command.args(&["--user", "--map-root-user", "--"]);
    command.arg("ostree");
    command.arg("show");
    command.arg("--repo");
    command.arg(repo_path);
    command.arg(format!("--print-metadata-key={}", key));
    command.arg(commit);

    let output = command.output()?;
    if output.status.success() {
        parse_metadata_list_output(&output.stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No such metadata key") {
            Ok(Vec::new())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Failed to read metadata {} from {}: {}",
                    key, commit, stderr
                ),
            ))
        }
    }
}

fn parse_metadata_list_output(raw: &[u8]) -> io::Result<Vec<String>> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let value = String::from_utf8(raw.to_vec())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut trimmed = value.trim().to_string();
    if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
        trimmed = trimmed[1..trimmed.len() - 1].to_string();
    }
    if trimmed.is_empty() {
        Ok(Vec::new())
    } else {
        serde_json::from_str(&trimmed).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid JSON metadata '{}': {}", trimmed, e),
            )
        })
    }
}

fn update_manifest_outputs(manifest_path: &str, suggestions: &RuntimeScanResult) -> io::Result<()> {
    let contents = fs::read_to_string(manifest_path)?;
    let mut doc: Value = serde_yaml::from_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let outputs_value = doc.get_mut("outputs").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Manifest missing outputs section",
        )
    })?;
    let outputs_map = outputs_value
        .as_mapping_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "outputs is not a mapping"))?;

    let mut changed = false;

    for (key, value) in outputs_map.iter_mut() {
        let category = match key.as_str() {
            Some(s) => s.to_string(),
            None => continue,
        };

        let new_requires: Vec<String> = suggestions
            .category_resolved(&category)
            .map(|commits| commits.keys().cloned().collect())
            .unwrap_or_default();

        let (mapping, converted) = ensure_output_mapping(value)?;
        if converted {
            changed = true;
        }
        if update_requires_field(mapping, &new_requires) {
            changed = true;
        }
        normalize_output_keys(mapping);
    }

    if !changed {
        println!(
            "No outputs.requires changes were necessary for {}",
            manifest_path
        );
        return Ok(());
    }

    let outputs_block = serialize_outputs_section(outputs_value)?;
    let (start, end) = locate_outputs_block(&contents)?;
    let mut new_contents = String::new();
    new_contents.push_str(&contents[..start]);
    new_contents.push_str(&outputs_block);
    if !outputs_block.ends_with('\n')
        && (end >= contents.len() || contents[start..end].contains('\n'))
    {
        new_contents.push('\n');
    }
    new_contents.push_str(&contents[end..]);
    fs::write(manifest_path, new_contents)?;
    println!(
        "Updated outputs.requires entries based on runtime scan in {}",
        manifest_path
    );

    Ok(())
}

fn ensure_output_mapping(value: &mut Value) -> io::Result<(&mut Mapping, bool)> {
    match value {
        Value::Mapping(map) => Ok((map, false)),
        Value::Sequence(_) | Value::Null => {
            let files_seq = if let Value::Sequence(seq) = value {
                mem::take(seq)
            } else {
                Vec::new()
            };
            let mut mapping = Mapping::new();
            mapping.insert(
                Value::String("files".to_string()),
                Value::Sequence(files_seq),
            );
            *value = Value::Mapping(mapping);
            if let Value::Mapping(map) = value {
                Ok((map, true))
            } else {
                unreachable!()
            }
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Output entry must be a sequence or mapping",
        )),
    }
}

fn update_requires_field(mapping: &mut Mapping, new_values: &[String]) -> bool {
    let key = Value::String("requires".to_string());
    let current: Vec<String> = mapping
        .get(&key)
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if new_values.is_empty() {
        if mapping.remove(&key).is_some() && !current.is_empty() {
            return true;
        }
        return false;
    }

    if current == new_values {
        return false;
    }

    let seq = Value::Sequence(
        new_values
            .iter()
            .map(|val| Value::String(val.clone()))
            .collect(),
    );
    mapping.insert(key, seq);
    true
}

fn normalize_output_keys(mapping: &mut Mapping) {
    let mut entries: Vec<(Value, Value)> = Vec::new();
    for key in ["files", "requires", "suggests"] {
        let key_value = Value::String(key.to_string());
        if let Some(value) = mapping.remove(&key_value) {
            entries.push((Value::String(key.to_string()), value));
        }
    }
    for (k, v) in mapping.iter() {
        entries.push((k.clone(), v.clone()));
    }
    mapping.clear();
    for (k, v) in entries {
        mapping.insert(k, v);
    }
}

fn serialize_outputs_section(outputs_value: &Value) -> io::Result<String> {
    let mapping = outputs_value
        .as_mapping()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "outputs is not a mapping"))?;
    let mut inner = serde_yaml::to_string(&Value::Mapping(mapping.clone()))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if inner.starts_with("---\n") {
        inner = inner[4..].to_string();
    } else if inner.starts_with("---") {
        inner = inner.trim_start_matches("---").trim_start().to_string();
    }
    if inner.ends_with("\n...\n") {
        inner.truncate(inner.len() - 5);
    } else if inner.ends_with("\n...") {
        inner.truncate(inner.len() - 4);
    }
    inner = inner.trim_end().to_string();

    let mut block = String::from("outputs:\n");
    for line in inner.lines() {
        if line.is_empty() {
            block.push('\n');
        } else {
            block.push_str("  ");
            block.push_str(line);
            block.push('\n');
        }
    }

    Ok(block)
}

fn locate_outputs_block(contents: &str) -> io::Result<(usize, usize)> {
    let mut start = None;
    let mut end = contents.len();
    let mut line_start = 0;

    while line_start < contents.len() {
        let line_end = contents[line_start..]
            .find('\n')
            .map(|idx| line_start + idx + 1)
            .unwrap_or(contents.len());
        let line = &contents[line_start..line_end];
        let trimmed = line.trim_end();

        if start.is_none() {
            if !line.starts_with(' ') && !line.starts_with('\t') && trimmed == "outputs:" {
                start = Some(line_start);
            }
        } else {
            let trimmed_ws = trimmed.trim();
            let is_top_level =
                !line.starts_with(' ') && !line.starts_with('\t') && !trimmed_ws.is_empty();
            if is_top_level && trimmed_ws != "outputs:" {
                end = line_start;
                break;
            }
        }

        if line_end == contents.len() {
            break;
        }
        line_start = line_end;
    }

    if let Some(start_idx) = start {
        Ok((start_idx, end))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unable to locate outputs section",
        ))
    }
}

fn apply_runtime_requires(manifest: &mut Manifest, suggestions: &RuntimeScanResult) {
    for (category, spec) in manifest.outputs.iter_mut() {
        let new_requires: Vec<String> = suggestions
            .category_resolved(category)
            .map(|commits| commits.keys().cloned().collect())
            .unwrap_or_default();
        spec.requires = new_requires;
    }
}

fn commit_bundle(
    repo_path: &str,
    bundle_name: &str,
    bundle: &Bundle,
    manifest: &Manifest,
) -> io::Result<()> {
    println!("Creating bundle: {}", bundle_name);

    let temp_dir = TempDir::new()?;
    let temp_dir_path = temp_dir.path();

    for output in &bundle.includes {
        let branch_name = format!(
            "x86_64/{}/{}/{}/outputs/{}",
            manifest.package.slug, manifest.package.version, manifest.package.flavor, output
        );
        checkout_ostree_into(repo_path, &branch_name, temp_dir_path, true)?;
    }

    let bundle_branch = format!(
        "x86_64/{}/{}/{}/bundles/{}",
        manifest.package.slug, manifest.package.version, manifest.package.flavor, bundle_name
    );

    let metadata = bundle_branch_metadata(manifest, bundle)?;

    commit_to_ostree(temp_dir_path, &bundle_branch, repo_path, &metadata)?;

    Ok(())
}

fn fetch_and_verify_input(input_spec: &Source, download_dir: &str) -> io::Result<PathBuf> {
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

fn calculate_output_checksum(output_dir: &Path) -> io::Result<String> {
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

fn categorize_files(rootfs_dir: &Path) -> HashMap<String, Vec<String>> {
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

fn print_outputs(
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use tempfile::NamedTempFile;

    fn base_manifest() -> &'static str {
        r#"
package:
  schema: 1
  name: sample
  slug: sample
  flavor: bootstrap/phase0
  version: "1.0"
dependencies: []
sources: []
build:
  script: "true"
outputs: {}
"#
    }

    #[test]
    fn parses_detailed_bundle_with_metadata() {
        let yaml = format!(
            "{base}bundles:\n  dev:\n    includes:\n      - bin\n      - lib\n    requires:\n      - x86_64/foo/1.0/outputs/lib\n    suggests:\n      - x86_64/bar/2.0/bundles/dev\n",
            base = base_manifest()
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let bundle = manifest.bundles.get("dev").unwrap();
        assert_eq!(bundle.includes, vec!["bin".to_string(), "lib".to_string()]);
        assert_eq!(
            bundle.requires,
            vec!["x86_64/foo/1.0/outputs/lib".to_string()]
        );
        assert_eq!(
            bundle.suggests,
            vec!["x86_64/bar/2.0/bundles/dev".to_string()]
        );
    }

    #[test]
    fn parses_detailed_output_with_metadata() {
        let yaml = format!(
            "{}\nbundles:\n  dev:\n    includes:\n      - bin",
            base_manifest().replacen(
                "outputs: {}",
                "outputs:\n  bin:\n    files:\n      - /usr/bin/foo\n    requires:\n      - x86_64/libfoo/1.0/outputs/lib\n    suggests:\n      - x86_64/foo-doc/1.0/outputs/doc",
                1,
            )
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let output = manifest.outputs.get("bin").unwrap();
        assert_eq!(output.files, vec!["/usr/bin/foo".to_string()]);
        assert_eq!(
            output.requires,
            vec!["x86_64/libfoo/1.0/outputs/lib".to_string()]
        );
        assert_eq!(
            output.suggests,
            vec!["x86_64/foo-doc/1.0/outputs/doc".to_string()]
        );
    }

    #[test]
    fn parses_legacy_output_format() {
        let yaml = format!(
            "{}\nbundles:\n  dev:\n    includes:\n      - bin",
            base_manifest().replacen("outputs: {}", "outputs:\n  bin:\n    - /usr/bin/foo", 1)
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let output = manifest.outputs.get("bin").unwrap();
        assert_eq!(output.files, vec!["/usr/bin/foo".to_string()]);
        assert!(output.requires.is_empty());
        assert!(output.suggests.is_empty());
    }

    #[test]
    fn parses_legacy_bundle_format() {
        let yaml = format!(
            "{base}bundles:\n  dev:\n    - bin\n    - lib\n",
            base = base_manifest()
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
        let bundle = manifest.bundles.get("dev").unwrap();
        assert_eq!(bundle.includes, vec!["bin".to_string(), "lib".to_string()]);
        assert!(bundle.requires.is_empty());
        assert!(bundle.suggests.is_empty());
    }

    #[test]
    fn closure_resolution_collects_all_commits() {
        let deps = vec![
            Dependency {
                commit: "pkg/A".into(),
                name: None,
            },
            Dependency {
                commit: "pkg/D".into(),
                name: None,
            },
        ];
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
        graph.insert("pkg/A", vec!["pkg/B", "pkg/C"]);
        graph.insert("pkg/B", vec!["pkg/C"]);
        graph.insert("pkg/C", vec!["pkg/D"]);
        graph.insert("pkg/D", vec![]);

        let result = resolve_dependency_closure_with_fetch(&deps, |commit| {
            Ok(graph
                .get(commit)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|s| s.to_string())
                .collect())
        })
        .unwrap();
        let expected: HashSet<String> = ["pkg/A", "pkg/B", "pkg/C", "pkg/D"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let actual: HashSet<String> = result.into_iter().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn closure_resolution_errors_on_cycle() {
        let deps = vec![Dependency {
            commit: "pkg/A".into(),
            name: None,
        }];
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
        graph.insert("pkg/A", vec!["pkg/B"]);
        graph.insert("pkg/B", vec!["pkg/A"]);

        let err = resolve_dependency_closure_with_fetch(&deps, |commit| {
            Ok(graph
                .get(commit)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|s| s.to_string())
                .collect())
        })
        .expect_err("expected cycle to be detected");
        assert!(
            err.to_string().contains("pkg/A -> pkg/B -> pkg/A"),
            "unexpected error message: {}",
            err
        );
    }

    #[test]
    fn detects_system_manifest_kind() {
        let yaml = r#"
schema: 1
kind: system
system:
  name: Demo
  slug: demo
  version: "1.0"
packages:
  - commit: x86_64/foo/1.0/base/bundles/dev
build:
  script: ":"
sources: []
dependencies: []
"#;
        let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(detect_manifest_kind(&value), ManifestKind::System);
        let sys: SystemManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(sys.system.slug, "demo");
        assert_eq!(sys.system.version, "1.0");
    }

    #[test]
    fn load_manifest_parses_system_kind() {
        let yaml = r#"
schema: 1
kind: system
system:
  name: Demo
  slug: demo
  version: "1.0"
packages:
  - commit: x86_64/foo/1.0/base/bundles/dev
build:
  script: ":"
sources: []
dependencies: []
"#;
        let mut file = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut file, yaml.as_bytes()).unwrap();
        match load_manifest(file.path().to_str().unwrap()).unwrap() {
            ManifestData::System(sys) => {
                assert_eq!(sys.system.slug, "demo");
                assert_eq!(sys.packages.len(), 1);
            }
            _ => panic!("Expected system manifest"),
        }
    }
}
