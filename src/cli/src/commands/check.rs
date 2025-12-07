use clap::Args;
use std::io;

use crate::deps::resolve_dependency_closure;
use crate::manifest::{load_manifest, ManifestData, ManifestIndex};

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

        // check 2: no bootstrap dependencies in resolution
        let manifest_data = load_manifest(file)?;

        if let ManifestData::Package(ref manifest) = manifest_data {
            // build a set of bootstrap dependency names
            let bootstrap_deps: std::collections::HashSet<&str> = manifest
                .dependencies
                .iter()
                .filter(|dep| dep.commit.contains("/bootstrap/"))
                .filter_map(|dep| dep.name.as_deref())
                .collect();

            // check resolution values
            for (file_path, dep_name) in &manifest.resolution {
                if dep_name == "self" {
                    continue;
                }
                if bootstrap_deps.contains(dep_name.as_str()) {
                    eprintln!(
                        "  error: resolution '{}' -> '{}' points to bootstrap dependency",
                        file_path, dep_name
                    );
                    has_errors = true;
                }
            }
        }

        // check 3: no package identity cycles in dependency chain
        if let ManifestData::Package(ref manifest) = manifest_data {
            if let Err(e) = resolve_dependency_closure(&manifest.dependencies, &manifest_index) {
                eprintln!("  error: {}", e);
                has_errors = true;
            }
        }
    }

    if has_errors {
        Err(io::Error::new(io::ErrorKind::Other, "check failed"))
    } else {
        println!("all checks passed");
        Ok(())
    }
}
