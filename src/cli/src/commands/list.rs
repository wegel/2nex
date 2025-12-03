use clap::Args;
use std::collections::HashMap;
use std::io;

use crate::repo::resolve_repo_path;
use crate::store::Store;

#[derive(Args)]
pub struct ListArgs {
    /// Repository path (auto-detected if not specified)
    #[clap(long)]
    pub repo: Option<String>,

    /// Show only assemblies (runtime-ready packages)
    #[clap(long)]
    pub assemblies: bool,

    /// Show only specific namespace (e.g., "cli/shells", "core/userland")
    #[clap(long)]
    pub namespace: Option<String>,

    /// Show full ref paths instead of summarized view
    #[clap(long)]
    pub full: bool,
}

pub fn run(args: &ListArgs) -> io::Result<()> {
    let repo_path = resolve_repo_path(args.repo.as_deref())?;
    let store = Store::open(&repo_path)?;
    let ref_list = store.refs(None)?;
    let refs = ref_list.join("\n");
    let mut packages: HashMap<String, PackageInfo> = HashMap::new();

    for line in refs.lines() {
        // parse: x86_64/pkg/<namespace>/<slug>/<version>/<type>/<name>
        let parts: Vec<&str> = line.split('/').collect();

        if parts.len() < 6 || parts[0] != "x86_64" || parts[1] != "pkg" {
            continue;
        }

        // find boundary (outputs, bundles, assembly)
        let boundary = parts
            .iter()
            .position(|p| *p == "outputs" || *p == "bundles" || *p == "assembly" || *p == "deploy");

        let boundary = match boundary {
            Some(b) => b,
            None => continue,
        };

        if boundary < 4 {
            continue;
        }

        let namespace = parts[2..boundary - 2].join("/");
        let slug = parts[boundary - 2];
        let version = parts[boundary - 1];
        let ref_type = parts[boundary];

        // filter by namespace if specified
        if let Some(ref ns_filter) = args.namespace {
            if !namespace.starts_with(ns_filter) {
                continue;
            }
        }

        // filter assemblies only if requested
        if args.assemblies && ref_type != "assembly" && ref_type != "deploy" {
            continue;
        }

        let key = format!("{}/{}/{}", namespace, slug, version);
        let info = packages.entry(key.clone()).or_insert_with(|| PackageInfo {
            namespace: namespace.to_string(),
            slug: slug.to_string(),
            version: version.to_string(),
            has_outputs: false,
            has_bundles: false,
            has_assembly: false,
            outputs: vec![],
            bundles: vec![],
        });

        match ref_type {
            "outputs" => {
                info.has_outputs = true;
                if parts.len() > boundary + 1 {
                    info.outputs.push(parts[boundary + 1].to_string());
                }
            }
            "bundles" => {
                info.has_bundles = true;
                if parts.len() > boundary + 1 {
                    info.bundles.push(parts[boundary + 1].to_string());
                }
            }
            "assembly" | "deploy" => {
                info.has_assembly = true;
            }
            _ => {}
        }
    }

    if args.full {
        // full output: show all refs
        for line in refs.lines() {
            if args.assemblies {
                if line.contains("/assembly/") || line.contains("/deploy/") {
                    println!("{}", line);
                }
            } else {
                println!("{}", line);
            }
        }
    } else {
        // summarized output
        let mut sorted_packages: Vec<_> = packages.values().collect();
        sorted_packages.sort_by(|a, b| {
            a.namespace
                .cmp(&b.namespace)
                .then(a.slug.cmp(&b.slug))
                .then(a.version.cmp(&b.version))
        });

        for pkg in sorted_packages {
            let mut flags = vec![];
            if pkg.has_outputs {
                flags.push("outputs");
            }
            if pkg.has_bundles {
                flags.push("bundles");
            }
            if pkg.has_assembly {
                flags.push("assembly");
            }

            println!(
                "{}/{} {} [{}]",
                pkg.namespace,
                pkg.slug,
                pkg.version,
                flags.join(", ")
            );
        }
    }

    Ok(())
}

struct PackageInfo {
    namespace: String,
    slug: String,
    version: String,
    has_outputs: bool,
    has_bundles: bool,
    has_assembly: bool,
    outputs: Vec<String>,
    bundles: Vec<String>,
}
