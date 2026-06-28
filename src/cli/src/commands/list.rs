use clap::Args;
use std::io;
use std::path::Path;
use walkdir::WalkDir;

use crate::manifest::{load_manifest, ManifestData};

#[derive(Args)]
pub struct ListArgs {
    /// Manifest database path (default: /nex/db/pkg)
    #[clap(long, default_value = "/nex/db/pkg")]
    pub db: String,

    /// Show only specific namespace (e.g., "cli", "core/userland")
    #[clap(long)]
    pub namespace: Option<String>,

    /// Show full paths to manifest files
    #[clap(long)]
    pub full: bool,
}

pub fn run(args: &ListArgs) -> io::Result<()> {
    let db_path = Path::new(&args.db);
    if !db_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Manifest database not found: {}", args.db),
        ));
    }

    let mut packages: Vec<PackageInfo> = Vec::new();

    for entry in WalkDir::new(db_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() || path.extension().map_or(true, |e| e != "yaml") {
            continue;
        }

        // load manifest
        let manifest = match load_manifest(&path.to_string_lossy()) {
            Ok(ManifestData::Package(m)) => m,
            _ => continue,
        };

        let namespace = &manifest.package.namespace;
        let slug = &manifest.package.slug;

        // filter by namespace if specified
        if let Some(ref ns_filter) = args.namespace {
            if !namespace.starts_with(ns_filter) {
                continue;
            }
        }

        let bundles: Vec<String> = manifest.bundles.keys().cloned().collect();
        let outputs: Vec<String> = manifest.outputs.keys().cloned().collect();

        packages.push(PackageInfo {
            path: path.to_string_lossy().to_string(),
            namespace: namespace.clone(),
            slug: slug.clone(),
            version: manifest.package.version.clone(),
            bundles,
            outputs,
        });
    }

    // sort by namespace, slug, version
    packages.sort_by(|a, b| {
        a.namespace
            .cmp(&b.namespace)
            .then(a.slug.cmp(&b.slug))
            .then(a.version.cmp(&b.version))
    });

    for pkg in packages {
        if args.full {
            println!("{}", pkg.path);
        } else {
            let mut targets = Vec::new();
            for b in &pkg.bundles {
                targets.push(format!("bundles/{}", b));
            }
            for o in &pkg.outputs {
                targets.push(format!("outputs/{}", o));
            }
            let targets_str = if targets.is_empty() {
                String::new()
            } else {
                format!(" [{}]", targets.join(", "))
            };
            println!(
                "{}/{} {}{}",
                pkg.namespace, pkg.slug, pkg.version, targets_str
            );
        }
    }

    Ok(())
}

struct PackageInfo {
    path: String,
    namespace: String,
    slug: String,
    version: String,
    bundles: Vec<String>,
    outputs: Vec<String>,
}
