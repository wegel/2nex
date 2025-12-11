use clap::Args;
use std::collections::HashSet;
use std::io;

use crate::manifest::ManifestIndex;
use crate::repo::resolve_repo_path;
use crate::store::Store;

#[derive(Args)]
pub struct InfoArgs {
    /// Package reference (e.g., "bash", "cli/shells/bash", or full store ref)
    pub package: String,

    /// Repository path (auto-detected if not specified)
    #[clap(long)]
    pub repo: Option<String>,

    /// Show info for specific version
    #[clap(long)]
    pub version: Option<String>,

    /// List available versions only
    #[clap(long)]
    pub versions: bool,

    /// Show dependency tree
    #[clap(long)]
    pub deps: bool,
}

pub fn run(args: &InfoArgs) -> io::Result<()> {
    let repo_path = resolve_repo_path(args.repo.as_deref())?;

    // find matching refs
    let refs = find_matching_refs(&repo_path, &args.package, args.version.as_deref())?;

    if refs.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("No package found matching '{}'", args.package),
        ));
    }

    // --versions: just list available versions
    if args.versions {
        return show_versions(&refs, &args.package);
    }

    // --deps: show dependency tree
    if args.deps {
        return show_dependency_tree(&repo_path, &refs);
    }

    // default: show full info grouped by package/version
    let mut current_pkg = String::new();
    for ref_path in &refs {
        let pkg_key = extract_package_key(ref_path);
        if pkg_key != current_pkg {
            if !current_pkg.is_empty() {
                println!();
            }
            current_pkg = pkg_key.clone();
            println!("Package: {}", pkg_key);
        }

        // determine ref type
        let ref_type = if ref_path.contains("/outputs/") {
            "output"
        } else if ref_path.contains("/bundles/") {
            "bundle"
        } else if ref_path.contains("/assembly/") || ref_path.contains("/deploy/") {
            "assembly"
        } else {
            "unknown"
        };

        let ref_name = ref_path.split('/').next_back().unwrap_or("?");
        println!("  {}: {}", ref_type, ref_name);

        // show metadata for this ref
        show_ref_metadata(&repo_path, ref_path)?;
    }

    Ok(())
}

fn show_versions(refs: &[String], query: &str) -> io::Result<()> {
    let mut versions: Vec<(String, String)> = vec![];
    let mut seen: HashSet<String> = HashSet::new();

    for ref_path in refs {
        let parts: Vec<&str> = ref_path.split('/').collect();
        let boundary = parts
            .iter()
            .position(|p| *p == "outputs" || *p == "bundles" || *p == "assembly" || *p == "deploy");

        if let Some(b) = boundary {
            if b >= 4 {
                let namespace = parts[2..b - 2].join("/");
                let slug = parts[b - 2];
                let version = parts[b - 1];
                let key = format!("{}/{}/{}", namespace, slug, version);
                if !seen.contains(&key) {
                    seen.insert(key);
                    versions.push((format!("{}/{}", namespace, slug), version.to_string()));
                }
            }
        }
    }

    if versions.is_empty() {
        println!("No versions found for '{}'", query);
    } else {
        println!("Versions of '{}':", query);
        for (pkg, ver) in &versions {
            println!("  {} {}", pkg, ver);
        }
    }

    Ok(())
}

fn show_dependency_tree(_repo: &str, refs: &[String]) -> io::Result<()> {
    // find an output ref to get dependencies from
    let dep_ref = refs
        .iter()
        .find(|r| r.contains("/outputs/"))
        .or_else(|| refs.iter().find(|r| r.contains("/bundles/")));

    let dep_ref = match dep_ref {
        Some(r) => r,
        None => {
            println!("No outputs or bundles found to show dependencies");
            return Ok(());
        }
    };

    let pkg_key = extract_package_key(dep_ref);
    println!(
        "Dependencies for {} (from manifest needs/resolution):",
        pkg_key
    );

    // load manifest index and show deps from manifest
    let manifest_index = match ManifestIndex::load("pkg") {
        Ok(idx) => idx,
        Err(e) => {
            println!("  (could not load manifests: {})", e);
            return Ok(());
        }
    };

    // parse ref to get namespace/slug
    let parts: Vec<&str> = dep_ref.split('/').collect();
    let pkg_idx = parts.iter().position(|&p| p == "pkg");
    let end_idx = parts.iter().position(|&p| p == "outputs" || p == "bundles");

    if let (Some(pkg_idx), Some(end_idx)) = (pkg_idx, end_idx) {
        if end_idx > pkg_idx + 2 {
            let slug_idx = end_idx - 2;
            let namespace = parts[pkg_idx + 1..slug_idx].join("/");
            let slug = parts[slug_idx];

            if let Some(manifest) = manifest_index.get_manifest(&namespace, slug) {
                print_manifest_deps(manifest, &manifest_index, 1, &mut HashSet::new());
            } else {
                println!("  (manifest not found for {}/{})", namespace, slug);
            }
        }
    }

    Ok(())
}

/// print dependencies from manifest needs/resolution
fn print_manifest_deps(
    manifest: &crate::manifest::types::Manifest,
    index: &ManifestIndex,
    depth: usize,
    visited: &mut HashSet<String>,
) {
    let indent = "  ".repeat(depth);

    // collect unique dependency names from resolution (skip self)
    let mut dep_names: Vec<&String> = manifest
        .resolution
        .values()
        .filter(|v| *v != "self")
        .collect();
    dep_names.sort();
    dep_names.dedup();

    if dep_names.is_empty() {
        println!("{}(no runtime dependencies)", indent);
        return;
    }

    for dep_name in dep_names {
        // find the dependency commit to get namespace/slug
        let dep = match manifest
            .dependencies
            .iter()
            .find(|d| d.name.as_deref() == Some(dep_name))
        {
            Some(d) => d,
            None => {
                println!("{}{} (not in dependencies list)", indent, dep_name);
                continue;
            }
        };

        // parse commit to get package info
        let parts: Vec<&str> = dep.commit.split('/').collect();
        let pkg_idx = parts.iter().position(|&p| p == "pkg");
        let end_idx = parts.iter().position(|&p| p == "outputs" || p == "bundles");

        if let (Some(pkg_idx), Some(end_idx)) = (pkg_idx, end_idx) {
            if end_idx > pkg_idx + 2 {
                let slug_idx = end_idx - 2;
                let namespace = parts[pkg_idx + 1..slug_idx].join("/");
                let slug = parts[slug_idx];
                let version = parts[end_idx - 1];

                println!(
                    "{}{} ({}/{}/{})",
                    indent, dep_name, namespace, slug, version
                );

                // avoid cycles
                let key = format!("{}/{}", namespace, slug);
                if visited.contains(&key) {
                    println!("{}  (cycle)", indent);
                    continue;
                }
                visited.insert(key.clone());

                // recursively show deps (only go 3 levels deep)
                if depth < 3 {
                    if let Some(dep_manifest) = index.get_manifest(&namespace, slug) {
                        print_manifest_deps(dep_manifest, index, depth + 1, visited);
                    }
                }
            }
        }
    }
}

fn find_matching_refs(repo: &str, query: &str, version: Option<&str>) -> io::Result<Vec<String>> {
    let store = Store::open(repo)?;
    let all_refs = store.refs(None)?;
    let mut matches: Vec<String> = vec![];

    // if query looks like a full ref, match exactly
    if query.starts_with("x86_64/") {
        for ref_name in &all_refs {
            if ref_name == query || ref_name.starts_with(&format!("{}/", query)) {
                matches.push(ref_name.to_string());
            }
        }
        return Ok(matches);
    }

    // otherwise search by slug or namespace/slug
    let query_parts: Vec<&str> = query.split('/').collect();

    for ref_name in &all_refs {
        let parts: Vec<&str> = ref_name.split('/').collect();
        if parts.len() < 6 || parts[0] != "x86_64" || parts[1] != "pkg" {
            continue;
        }

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
        let ver = parts[boundary - 1];

        // filter by version if specified
        if let Some(v) = version {
            if ver != v {
                continue;
            }
        }

        // match query
        let matched = if query_parts.len() == 1 {
            // just slug
            slug == query_parts[0]
        } else {
            // namespace/slug or partial namespace
            let full_path = format!("{}/{}", namespace, slug);
            full_path.contains(query) || full_path.ends_with(query)
        };

        if matched {
            matches.push(ref_name.to_string());
        }
    }

    matches.sort();
    Ok(matches)
}

fn extract_package_key(ref_path: &str) -> String {
    let parts: Vec<&str> = ref_path.split('/').collect();
    let boundary = parts
        .iter()
        .position(|p| *p == "outputs" || *p == "bundles" || *p == "assembly" || *p == "deploy")
        .unwrap_or(parts.len());

    if boundary >= 4 {
        parts[2..boundary].join("/")
    } else {
        ref_path.to_string()
    }
}

fn show_ref_metadata(repo: &str, ref_path: &str) -> io::Result<()> {
    let store = Store::open(repo)?;

    // try to read common metadata keys
    let metadata_keys = ["nex.manifest.hash"];

    for key in &metadata_keys {
        if let Ok(Some(value)) = store.get_metadata(ref_path, key) {
            if !value.is_empty() {
                println!("    {}: {}", key.strip_prefix("nex.").unwrap_or(key), value);
            }
        }
    }

    Ok(())
}
