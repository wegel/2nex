use clap::Args;
use std::io;

use crate::deps::resolve_dependency_closure;
use crate::manifest::types::Overlay;
use crate::manifest::{load_manifest, ManifestData, ManifestIndex};
use std::path::Path;

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
    #[clap(long, default_value = "pkg")]
    pub pkg_dir: String,
}

pub fn run(args: &CheckArgs) -> io::Result<()> {
    let mut has_errors = false;

    // load manifest index for dependency chain validation
    let manifest_index = ManifestIndex::load(&args.pkg_dir)?;

    for file in &args.files {
        println!("checking {}", file);

        // check 1: formatting
        let original = std::fs::read_to_string(file)?;
        let formatted = crate::manifest::format::format_manifest_string(&original)?;

        if original != formatted {
            eprintln!("  error: needs formatting");
            has_errors = true;
        }

        // check 2: no bootstrap dependencies unless seed package
        let manifest_data = load_manifest(file)?;

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

            for (file_path, dep_name) in &manifest.resolution {
                if dep_name == "self" {
                    continue;
                }
                if bootstrap_dep_names.contains(dep_name.as_str()) {
                    eprintln!(
                        "  error: resolution '{}' -> '{}' points to bootstrap dependency",
                        file_path, dep_name
                    );
                    has_errors = true;
                }
            }
        }

        // check 3: no forbidden usrmerge paths in system overlays
        if let ManifestData::System(ref manifest) = manifest_data {
            for overlay_path in &manifest.overlays {
                let overlay_path = Path::new(overlay_path);
                let overlay_content = std::fs::read_to_string(overlay_path).map_err(|e| {
                    io::Error::new(
                        e.kind(),
                        format!("failed to read {}: {}", overlay_path.display(), e),
                    )
                })?;

                let overlay: Overlay = serde_yaml::from_str(&overlay_content).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("failed to parse {}: {}", overlay_path.display(), e),
                    )
                })?;

                for entry in &overlay.files {
                    let path_str = entry.path.to_string_lossy();
                    if let Some(prefix) = forbidden_usrmerge_prefix(&path_str) {
                        if path_str != prefix {
                            eprintln!(
                                "  error: overlay '{}' places '{}' under forbidden usrmerge path '{}'",
                                overlay_path.display(),
                                path_str,
                                prefix
                            );
                            has_errors = true;
                            continue;
                        }

                        match entry.symlink.as_ref().and_then(|p| p.to_str()) {
                            Some(target) if is_allowed_usrmerge_symlink(&path_str, target) => {}
                            Some(target) => {
                                eprintln!(
                                    "  error: overlay '{}' has invalid symlink target '{}' for '{}'",
                                    overlay_path.display(),
                                    target,
                                    path_str
                                );
                                has_errors = true;
                            }
                            None => {
                                eprintln!(
                                    "  error: overlay '{}' must define a symlink for '{}'",
                                    overlay_path.display(),
                                    path_str
                                );
                                has_errors = true;
                            }
                        }
                    }
                }
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
