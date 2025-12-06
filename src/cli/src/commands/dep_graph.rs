use std::collections::{HashSet, VecDeque};
use std::io;

use clap::Args;

use crate::manifest::{load_manifest, ManifestData};

#[derive(Args)]
pub struct DepGraphArgs {
    /// Manifest file to analyze
    pub manifest: String,

    /// Only show refs matching this pattern (e.g., "bootstrap/")
    #[clap(long)]
    pub filter: Option<String>,
}

pub fn run(args: &DepGraphArgs) -> io::Result<()> {
    // BFS to collect all transitive dependencies (as commit refs)
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut all_refs: Vec<String> = Vec::new();

    // load initial manifest
    let start_manifest = load_manifest(&args.manifest)?;
    let start_deps = match &start_manifest {
        ManifestData::Package(m) => &m.dependencies,
        ManifestData::System(s) => &s.dependencies,
    };

    // seed queue with initial dependencies
    for dep in start_deps {
        if !visited.contains(&dep.commit) {
            visited.insert(dep.commit.clone());
            queue.push_back(dep.commit.clone());
        }
    }

    while let Some(commit_ref) = queue.pop_front() {
        all_refs.push(commit_ref.clone());

        // derive manifest path from commit ref
        if let Some(manifest_path) = ref_to_manifest_path(&commit_ref) {
            match load_manifest(&manifest_path) {
                Ok(manifest_data) => {
                    let deps = match &manifest_data {
                        ManifestData::Package(m) => &m.dependencies,
                        ManifestData::System(s) => &s.dependencies,
                    };

                    for dep in deps {
                        if !visited.contains(&dep.commit) {
                            visited.insert(dep.commit.clone());
                            queue.push_back(dep.commit.clone());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("warning: failed to load {}: {}", manifest_path, e);
                }
            }
        }
    }

    // output results
    for commit_ref in &all_refs {
        if let Some(ref filter) = args.filter {
            if commit_ref.contains(filter) {
                println!("{}", commit_ref);
            }
        } else {
            println!("{}", commit_ref);
        }
    }

    if args.filter.is_none() {
        eprintln!("\nTotal: {} dependencies in graph", all_refs.len());
    }

    Ok(())
}

/// Convert commit ref to manifest path
/// e.g., x86_64/pkg/bootstrap/phase3/glibc-bootstrap/2.39/bundles/dev -> pkg/bootstrap/phase3/glibc-bootstrap.yaml
fn ref_to_manifest_path(commit_ref: &str) -> Option<String> {
    let parts: Vec<&str> = commit_ref.split('/').collect();
    if parts.len() < 5 {
        return None;
    }

    // skip "x86_64" prefix if present
    let start_idx = if parts[0] == "x86_64" { 1 } else { 0 };

    // must start with "pkg"
    if parts.get(start_idx) != Some(&"pkg") {
        return None;
    }

    // find where version ends (before outputs/bundles/files)
    let mut end_idx = parts.len();
    for (i, part) in parts.iter().enumerate().skip(start_idx) {
        if *part == "outputs" || *part == "bundles" || *part == "files" {
            end_idx = i;
            break;
        }
    }

    // need at least pkg/<namespace>/<slug>/<version>
    if end_idx < start_idx + 3 {
        return None;
    }

    // slug is second to last, version is last before outputs/bundles/files
    let slug = parts[end_idx - 2];
    let namespace_parts = &parts[start_idx..end_idx - 2]; // includes "pkg"

    Some(format!("{}/{}.yaml", namespace_parts.join("/"), slug))
}
