use clap::Args;
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::process::Command;

use crate::deps::{resolve_dependency_closure, resolve_dependency_closure_with_providers};
use crate::manifest::types::{ManifestSource, Source, SystemManifest};
use crate::manifest::{load_manifest_from_source, Manifest, ManifestData, ManifestIndex};
use crate::refs::{PackageRef, RefType};
use crate::system::dependencies_from_system_packages;
use std::path::Path;
use std::path::PathBuf;

const USRMERGE_FORBIDDEN_PREFIXES: [&str; 5] = ["/bin", "/sbin", "/lib", "/lib64", "/usr/sbin"];

fn matches_usrmerge_prefix(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{}/", prefix))
}

fn forbidden_usrmerge_prefix(path: &str) -> Option<&'static str> {
    USRMERGE_FORBIDDEN_PREFIXES
        .iter()
        .copied()
        .find(|prefix| matches_usrmerge_prefix(path, prefix))
}

fn is_allowed_usrmerge_symlink(path: &str, target: &str) -> bool {
    let normalized_target = target.trim_end_matches('/');
    match path {
        "/bin" => matches!(normalized_target, "/usr/bin" | "usr/bin"),
        "/sbin" => matches!(
            normalized_target,
            "/usr/bin" | "usr/bin" | "/usr/sbin" | "usr/sbin"
        ),
        "/lib" => matches!(normalized_target, "/usr/lib" | "usr/lib"),
        "/lib64" => matches!(
            normalized_target,
            "/usr/lib" | "usr/lib" | "/usr/lib64" | "usr/lib64"
        ),
        "/usr/sbin" => matches!(normalized_target, "/usr/bin" | "usr/bin"),
        _ => false,
    }
}

#[derive(Args)]
pub struct CheckArgs {
    /// Manifest file(s) to check
    #[clap(required = true)]
    pub files: Vec<String>,

    /// Directory containing manifests for dependency resolution
    #[clap(long)]
    pub pkg_dir: Option<PathBuf>,
}

pub fn run(args: &CheckArgs) -> io::Result<()> {
    let mut has_errors = false;

    let manifest_dirs = check_manifest_dirs(args)?;
    let manifest_index = ManifestIndex::load_many(&manifest_dirs)?;

    for file in &args.files {
        println!("checking {}", file);

        // check 1: formatting
        let original = std::fs::read_to_string(file)?;
        let formatted = crate::manifest::format::format_manifest_string(&original)?;

        if original != formatted {
            eprintln!("  error: needs formatting");
            has_errors = true;
        }

        for error in validate_declared_sources(Path::new(file), &original)? {
            eprintln!("  error: {}", error);
            has_errors = true;
        }

        // check 2: no bootstrap dependencies unless seed package
        let manifest_path = Path::new(file).canonicalize()?;
        let manifest_data = load_manifest_from_source(&ManifestSource::Path(manifest_path))?;

        for error in validate_manifest_refs(&manifest_data) {
            eprintln!("  error: {}", error);
            has_errors = true;
        }

        if let ManifestData::Package(ref manifest) = manifest_data {
            // check 2a: no forbidden usrmerge paths in outputs
            for (output_name, output) in &manifest.outputs {
                for entry in &output.files {
                    if let Some(prefix) = forbidden_usrmerge_prefix(&entry.path) {
                        eprintln!(
                            "  error: output '{}' contains forbidden usrmerge path '{}'",
                            output_name, entry.path
                        );
                        eprintln!("         (prefix '{}' is reserved for symlinks)", prefix);
                        has_errors = true;
                    }
                }
            }

            // find all bootstrap dependencies
            let bootstrap_deps: Vec<&str> = manifest
                .dependencies
                .iter()
                .filter(|dep| dep.commit.contains("/bootstrap/"))
                .map(|dep| dep.commit.as_str())
                .collect();

            // error if non-seed package has bootstrap dependencies
            if !manifest.package.seed && !bootstrap_deps.is_empty() {
                for dep in &bootstrap_deps {
                    eprintln!(
                        "  error: bootstrap dependency '{}' not allowed (missing seed: true)",
                        dep
                    );
                }
                has_errors = true;
            }

            // also check resolution values don't point to bootstrap deps
            let bootstrap_dep_names: std::collections::HashSet<&str> = manifest
                .dependencies
                .iter()
                .filter(|dep| dep.commit.contains("/bootstrap/"))
                .filter_map(|dep| dep.name.as_deref())
                .collect();

            for (file_path, target) in &manifest.resolution {
                if target.is_self() {
                    continue;
                }
                let Some(dep_name) = target.dependency_name() else {
                    continue;
                };
                if bootstrap_dep_names.contains(dep_name) {
                    eprintln!(
                        "  error: resolution '{}' -> '{}' points to bootstrap dependency",
                        file_path, dep_name
                    );
                    has_errors = true;
                }
            }
        }

        // check 3: no forbidden usrmerge paths in assembly file entries
        if let ManifestData::System(ref manifest) = manifest_data {
            for entry in &manifest.files {
                let path_str = entry.path.to_string_lossy();
                let Some(prefix) = forbidden_usrmerge_prefix(&path_str) else {
                    continue;
                };
                if path_str != prefix {
                    eprintln!(
                        "  error: file entry '{}' is under forbidden usrmerge path '{}'",
                        path_str, prefix
                    );
                    has_errors = true;
                    continue;
                }

                match entry.symlink.as_ref().and_then(|p| p.to_str()) {
                    Some(target) if is_allowed_usrmerge_symlink(&path_str, target) => {}
                    Some(target) => {
                        eprintln!(
                            "  error: file entry '{}' has invalid symlink target '{}'",
                            path_str, target
                        );
                        has_errors = true;
                    }
                    None => {
                        eprintln!("  error: file entry '{}' must define a symlink", path_str);
                        has_errors = true;
                    }
                }
            }
            if let Err(error) = validate_system_providers(manifest, &manifest_index) {
                eprintln!("  error: {}", error);
                has_errors = true;
            }
            let package_dependency_specs = dependencies_from_system_packages(&manifest.packages);
            if let Err(error) = resolve_dependency_closure_with_providers(
                &package_dependency_specs,
                &manifest_index,
                &manifest.providers,
            ) {
                eprintln!("  error: {}", error);
                has_errors = true;
            }
        }

        // check 4: no package identity cycles in dependency chain
        if let ManifestData::Package(ref manifest) = manifest_data {
            if let Err(e) = resolve_dependency_closure(&manifest.dependencies, &manifest_index) {
                eprintln!("  error: {}", e);
                has_errors = true;
            }
        }
    }

    if has_errors {
        Err(io::Error::other("check failed"))
    } else {
        println!("all checks passed");
        Ok(())
    }
}

fn validate_manifest_refs(manifest: &ManifestData) -> Vec<String> {
    let mut errors = Vec::new();
    let dependencies = match manifest {
        ManifestData::Package(package) => package.dependencies.as_slice(),
        ManifestData::System(system) => system.dependencies.as_slice(),
    };
    for dependency in dependencies {
        validate_manifest_ref(
            dependency.manifest_ref.as_deref(),
            &dependency.commit,
            "dependency",
            &mut errors,
        );
    }
    if let ManifestData::System(system) = manifest {
        for package in &system.packages {
            validate_manifest_ref(
                package.manifest_ref.as_deref(),
                &package.commit,
                "assembly package",
                &mut errors,
            );
        }
    }
    errors
}

fn validate_manifest_ref(
    manifest_ref: Option<&str>,
    commit: &str,
    kind: &str,
    errors: &mut Vec<String>,
) {
    let Some(manifest_ref) = manifest_ref else {
        return;
    };
    if !is_git_object_id(manifest_ref) {
        errors.push(format!(
            "{} '{}' manifest_ref must be a full lowercase Git object ID",
            kind, commit
        ));
    }
}

fn is_git_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate_declared_sources(manifest_path: &Path, content: &str) -> io::Result<Vec<String>> {
    let document: serde_yaml::Value = serde_yaml::from_str(content)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let sources = match document.get("sources") {
        Some(value) if !value.is_null() => serde_yaml::from_value::<Vec<Source>>(value.clone())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
        _ => Vec::new(),
    };
    let repository_root = crate::manifest::repository_root_for_path(manifest_path)?;
    let mut errors = Vec::new();

    for source in &sources {
        validate_source_shape(source, &mut errors);
        validate_source_paths(source, &repository_root, &mut errors);
    }
    Ok(errors)
}

fn validate_source_shape(source: &Source, errors: &mut Vec<String>) {
    let selector_count = [
        source.url.is_some(),
        source.file.is_some(),
        source.dev.is_some(),
        source.cargo_lock.is_some(),
        source.go_sum.is_some(),
        source.zig_zon.is_some(),
        source.git_bundle.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if selector_count != 1 {
        errors.push(format!(
            "source '{}' must set exactly one of url, file, dev, cargo_lock, go_sum, zig_zon, or git_bundle",
            source.name
        ));
    }
    if source.cargo_toml.is_some() && source.cargo_lock.is_none() {
        errors.push(format!(
            "source '{}' may set cargo_toml only with cargo_lock",
            source.name
        ));
    }
    if source.file.as_deref().is_some_and(is_remote_reference) {
        errors.push(format!(
            "source '{}' must use url, not file, for an HTTP address",
            source.name
        ));
    }

    if source.dev.is_none() {
        match source.sha256.as_deref() {
            Some(hash)
                if hash.len() == 64
                    && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                    && hash.bytes().all(|byte| !byte.is_ascii_uppercase()) => {}
            _ => errors.push(format!(
                "source '{}' must set sha256 to 64 lowercase hexadecimal characters",
                source.name
            )),
        }
    }
}

fn validate_source_paths(source: &Source, repository_root: &Path, errors: &mut Vec<String>) {
    for (field, reference) in [
        ("file", source.file.as_deref()),
        ("cargo_lock", source.cargo_lock.as_deref()),
        ("cargo_toml", source.cargo_toml.as_deref()),
        ("go_sum", source.go_sum.as_deref()),
        ("zig_zon", source.zig_zon.as_deref()),
    ]
    .into_iter()
    .filter_map(|(field, reference)| reference.map(|reference| (field, reference)))
    .filter(|(_, reference)| !is_remote_reference(reference))
    {
        let path = Path::new(reference);
        if path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
        {
            errors.push(format!(
                "source '{}' {} path must stay relative to its repository: {}",
                source.name, field, reference
            ));
            continue;
        }

        let full_path = repository_root.join(path);
        let canonical = match full_path.canonicalize() {
            Ok(path) if path.is_file() && path == full_path => path,
            Ok(_) => {
                errors.push(format!(
                    "source '{}' {} path must be a regular file without symlinks: {}",
                    source.name, field, reference
                ));
                continue;
            }
            Err(error) => {
                errors.push(format!(
                    "source '{}' {} path cannot be read: {} ({})",
                    source.name, field, reference, error
                ));
                continue;
            }
        };
        let Ok(_) = canonical.strip_prefix(repository_root) else {
            errors.push(format!(
                "source '{}' {} path escapes its repository: {}",
                source.name, field, reference
            ));
            continue;
        };
        if !git_tracks(repository_root, path) {
            errors.push(format!(
                "source '{}' {} path is not tracked by Git: {}",
                source.name, field, reference
            ));
            continue;
        }

        if field == "file" {
            validate_local_file_hash(source, &canonical, errors);
        }
    }
}

fn validate_local_file_hash(source: &Source, path: &Path, errors: &mut Vec<String>) {
    let Some(expected) = source.sha256.as_deref() else {
        return;
    };
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            errors.push(format!(
                "source '{}' file cannot be read: {} ({})",
                source.name,
                path.display(),
                error
            ));
            return;
        }
    };
    let actual = hex::encode(Sha256::digest(bytes));
    if actual != expected {
        errors.push(format!(
            "source '{}' file sha256 mismatch: expected {}, got {}",
            source.name, expected, actual
        ));
    }
}

fn git_tracks(repository_root: &Path, relative_path: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["ls-files", "--error-unmatch", "--"])
        .arg(relative_path)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn is_remote_reference(reference: &str) -> bool {
    reference.starts_with("http://") || reference.starts_with("https://")
}

fn check_manifest_dirs(args: &CheckArgs) -> io::Result<Vec<PathBuf>> {
    if let Some(directory) = &args.pkg_dir {
        return Ok(vec![directory.canonicalize()?]);
    }
    let first_file = args.files.first().expect("clap requires a manifest file");
    let repositories = crate::manifest::ManifestRepositories::discover(Path::new(first_file))?;
    Ok(repositories.package_dirs())
}

fn validate_system_providers(
    manifest: &SystemManifest,
    manifest_index: &ManifestIndex,
) -> io::Result<()> {
    for (capability, provider_ref) in &manifest.providers {
        validate_provider_ref(capability, provider_ref, manifest_index)?;
    }
    Ok(())
}

fn validate_provider_ref(
    capability: &str,
    provider_ref: &str,
    manifest_index: &ManifestIndex,
) -> io::Result<()> {
    let package_ref = PackageRef::parse(provider_ref).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("provider {capability} has invalid ref {provider_ref}: {error}"),
        )
    })?;
    let manifest = manifest_index
        .get_manifest(&package_ref.namespace, &package_ref.slug)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("provider {capability} ref {provider_ref} has no package manifest"),
            )
        })?;
    let output_names = provider_output_names(provider_ref, manifest, &package_ref)?;
    if output_names
        .iter()
        .any(|output_name| output_provides(manifest, output_name, capability))
    {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("provider {capability} ref {provider_ref} does not declare {capability}"),
    ))
}

fn provider_output_names(
    provider_ref: &str,
    manifest: &Manifest,
    package_ref: &PackageRef,
) -> io::Result<Vec<String>> {
    match &package_ref.ref_type {
        RefType::Output { name, .. } => Ok(vec![name.clone()]),
        RefType::Bundle { name, .. } => manifest
            .bundles
            .get(name)
            .map(|bundle| bundle.includes.clone())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("provider ref {provider_ref} names missing bundle {name}"),
                )
            }),
        RefType::Files { .. } => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("provider ref {provider_ref} must be an output or bundle ref"),
        )),
    }
}

fn output_provides(manifest: &Manifest, output_name: &str, capability: &str) -> bool {
    manifest.outputs.get(output_name).is_some_and(|output| {
        output
            .provides
            .iter()
            .any(|provided| provided == capability)
    })
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod check_tests;
