use clap::Args;
use std::collections::BTreeMap;
use std::io;

use crate::manifest::ManifestIndex;
use crate::materializer::{resolve_runtime_deps_precomputed, MaterializeRequest};
use crate::repo::{detect_manifest_dir, resolve_repo_path};
use crate::store::Store;

#[derive(Args)]
pub struct ResolveArgs {
    /// Package to resolve (e.g., "bash", "cli/shells/bash", or full ref)
    pub package: String,

    /// Repository path (auto-detected if not specified)
    #[clap(long)]
    pub repo: Option<String>,

    /// Show full dependency chain with reasons
    #[clap(long, short)]
    pub verbose: bool,

    /// Look up providers for a specific library (e.g., "libc.so.6")
    #[clap(long)]
    pub lookup: Option<String>,
}

pub fn run(args: &ResolveArgs) -> io::Result<()> {
    let repo_path = resolve_repo_path(args.repo.as_deref())?;

    // load manifest index (required for precomputed deps)
    let manifest_index = if let Some(manifest_dir) = detect_manifest_dir() {
        match ManifestIndex::load(&manifest_dir) {
            Ok(index) => {
                println!(
                    "Using manifest index: {} manifests, {} files",
                    index.manifest_count(),
                    index.file_count()
                );
                index
            }
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Manifest index required: {}", e),
                ));
            }
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No manifest directory found",
        ));
    };
    println!();

    // lookup mode: just show providers for a library
    if let Some(ref library) = args.lookup {
        let providers = manifest_index.resolve_all(library);
        if providers.is_empty() {
            println!("No providers found for: {}", library);
        } else {
            println!("Providers for {}:", library);
            for (commit, path) in providers {
                let priority = phase_priority(commit);
                println!("  {} (priority {})", commit, priority);
                println!("    provides: {}", path);
            }
            if let Some(best) = manifest_index.resolve(library) {
                println!();
                println!("Selected: {}", best);
            }
        }
        return Ok(());
    }

    // find the package ref
    let package_ref = find_package_ref(&repo_path, &args.package)?;
    println!("Resolving: {}", package_ref);
    println!();

    // resolve dependencies using precomputed deps
    let requests = vec![MaterializeRequest::Bundle {
        commit: package_ref.clone(),
    }];

    let closure = resolve_runtime_deps_precomputed(
        &repo_path,
        &requests,
        &manifest_index,
        &BTreeMap::new(),
        &[],
    )?;

    // print results
    println!("Runtime closure: {} commit(s)", closure.commits.len());
    println!();

    for commit in &closure.commits {
        let short = shorten_commit(commit);
        if args.verbose {
            println!("  {}", commit);
            if let Some(reasons) = closure.reasons.get(commit) {
                for reason in reasons.iter().take(5) {
                    println!("    <- {}", reason);
                }
                if reasons.len() > 5 {
                    println!("    ... and {} more", reasons.len() - 5);
                }
            }
        } else {
            println!("  {}", short);
        }
    }

    if closure.has_unresolved() {
        println!();
        println!("Unresolved dependencies:");
        for (dep, reasons) in &closure.unresolved {
            println!("  {} ({} references)", dep, reasons.len());
            if args.verbose {
                for reason in reasons.iter().take(3) {
                    println!("    <- {}", reason);
                }
            }
        }
    }

    Ok(())
}

fn find_package_ref(repo: &str, query: &str) -> io::Result<String> {
    let store = Store::open(repo)?;
    let all_refs = store.refs(None)?;
    let query_parts: Vec<&str> = query.split('/').collect();

    // prefer bundles/full, then bundles/*, then outputs/bin
    let priorities = ["/bundles/full", "/bundles/", "/outputs/bin"];

    for priority in &priorities {
        for ref_name in &all_refs {
            if !ref_name.contains(priority) {
                continue;
            }

            let parts: Vec<&str> = ref_name.split('/').collect();
            if parts.len() < 6 || parts[0] != "x86_64" || parts[1] != "pkg" {
                continue;
            }

            let boundary = parts
                .iter()
                .position(|p| *p == "bundles" || *p == "outputs");

            let boundary = match boundary {
                Some(b) => b,
                None => continue,
            };

            if boundary < 4 {
                continue;
            }

            let namespace = parts[2..boundary - 2].join("/");
            let slug = parts[boundary - 2];

            // match query
            let matched = if query_parts.len() == 1 {
                slug == query_parts[0]
            } else {
                let full_path = format!("{}/{}", namespace, slug);
                full_path.contains(query) || full_path.ends_with(query)
            };

            if matched {
                return Ok(ref_name.to_string());
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("No package found for '{}'", query),
    ))
}

/// Get phase priority from commit path (higher = later phase = higher priority).
fn phase_priority(commit: &str) -> u32 {
    if commit.contains("/bootstrap/phase0/") {
        0
    } else if commit.contains("/bootstrap/phase1/") {
        1
    } else if commit.contains("/bootstrap/phase2/") {
        2
    } else if commit.contains("/bootstrap/phase3/") {
        3
    } else {
        // non-bootstrap packages have highest priority
        100
    }
}

fn shorten_commit(commit: &str) -> String {
    // x86_64/pkg/libs/compression/bzip2/1.0.8/bundles/full -> libs/compression/bzip2/1.0.8
    let parts: Vec<&str> = commit.split('/').collect();
    if parts.len() < 6 {
        return commit.to_string();
    }

    let boundary = parts
        .iter()
        .position(|p| *p == "bundles" || *p == "outputs")
        .unwrap_or(parts.len());

    if boundary >= 4 {
        parts[2..boundary].join("/")
    } else {
        commit.to_string()
    }
}
