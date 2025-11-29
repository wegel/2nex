use clap::Args;
use std::collections::HashSet;
use std::io;
use std::process::Command;

use crate::repo::resolve_repo_path;

#[derive(Args)]
pub struct SearchArgs {
    /// Search query (matches slug, namespace, or version)
    pub query: String,

    /// OSTree repository path (auto-detected if not specified)
    #[clap(long)]
    pub repo: Option<String>,

    /// Case-sensitive search
    #[clap(long)]
    pub case_sensitive: bool,
}

pub fn run(args: &SearchArgs) -> io::Result<()> {
    let repo_path = resolve_repo_path(args.repo.as_deref())?;

    let output = Command::new("ostree")
        .args(["refs", "--repo", &repo_path])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "Failed to list refs"));
    }

    let refs = String::from_utf8_lossy(&output.stdout);
    let query = if args.case_sensitive {
        args.query.clone()
    } else {
        args.query.to_lowercase()
    };

    let mut seen: HashSet<String> = HashSet::new();
    let mut results: Vec<SearchResult> = vec![];

    for line in refs.lines() {
        let parts: Vec<&str> = line.split('/').collect();
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
        let version = parts[boundary - 1];

        // search in namespace and slug
        let search_text = if args.case_sensitive {
            format!("{}/{}", namespace, slug)
        } else {
            format!("{}/{}", namespace, slug).to_lowercase()
        };

        if !search_text.contains(&query) {
            continue;
        }

        let key = format!("{}/{}/{}", namespace, slug, version);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);

        results.push(SearchResult {
            namespace: namespace.to_string(),
            slug: slug.to_string(),
            version: version.to_string(),
        });
    }

    // sort results
    results.sort_by(|a, b| {
        a.namespace
            .cmp(&b.namespace)
            .then(a.slug.cmp(&b.slug))
            .then(a.version.cmp(&b.version))
    });

    if results.is_empty() {
        println!("No packages found matching '{}'", args.query);
    } else {
        println!(
            "Found {} package(s) matching '{}':",
            results.len(),
            args.query
        );
        println!();
        for r in &results {
            println!("  {}/{} {}", r.namespace, r.slug, r.version);
        }
    }

    Ok(())
}

struct SearchResult {
    namespace: String,
    slug: String,
    version: String,
}
