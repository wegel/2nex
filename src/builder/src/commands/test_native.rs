//! test command for native OSTree implementation.

use std::io;

use clap::Args;

use crate::ostree_native::OstreeRepo;
use crate::repo::resolve_repo_path;

#[derive(Args)]
pub struct TestNativeArgs {
    /// OSTree repository path (auto-detected if not specified)
    #[clap(long)]
    pub repo: Option<String>,

    /// ref or commit to test
    #[clap(default_value = "")]
    pub ref_name: String,
}

pub fn run(args: &TestNativeArgs) -> io::Result<()> {
    let repo_path = resolve_repo_path(args.repo.as_deref())?;

    println!("Testing native OSTree implementation...\n");
    println!("Repository: {}", repo_path);

    let repo = OstreeRepo::open(&repo_path)?;

    // list refs
    println!("\n=== Refs ===");
    let refs = repo.refs(None)?;
    println!("Total refs: {}", refs.len());
    for r in refs.iter().take(5) {
        println!("  {}", r);
    }
    if refs.len() > 5 {
        println!("  ... and {} more", refs.len() - 5);
    }

    // if a ref was specified, test more operations
    let test_ref = if args.ref_name.is_empty() {
        refs.first().cloned()
    } else {
        Some(args.ref_name.clone())
    };

    if let Some(ref_name) = test_ref {
        println!("\n=== Testing ref: {} ===", ref_name);

        // resolve ref
        let checksum = repo.resolve_ref(&ref_name)?;
        println!("Checksum: {}", checksum);

        // get commit info
        let info = repo.get_commit_info(&ref_name)?;
        println!("Root tree: {}", info.root_tree);
        println!("Root meta: {}", info.root_meta);
        println!("Subject: {}", info.subject);
        println!("Metadata:");
        for (k, v) in &info.metadata {
            println!("  {}: {}", k, v);
        }

        // list files
        println!("\n=== Files ===");
        let files = repo.ls(&ref_name)?;
        println!("Total entries: {}", files.len());
        for f in files.iter().take(20) {
            println!("  {}", f);
        }
        if files.len() > 20 {
            println!("  ... and {} more", files.len() - 20);
        }

        // test checkout to same filesystem as repo (hardlinks require this)
        println!("\n=== Testing checkout ===");
        let repo_parent = std::path::Path::new(&repo_path)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let checkout_path = repo_parent.join("native-checkout-test");
        if checkout_path.exists() {
            std::fs::remove_dir_all(&checkout_path)?;
        }
        std::fs::create_dir_all(&checkout_path)?;
        println!("Checking out to: {}", checkout_path.display());
        repo.checkout(&ref_name, &checkout_path, true)?;

        // count files in checkout
        let mut count = 0;
        for entry in walkdir::WalkDir::new(&checkout_path) {
            if entry.is_ok() {
                count += 1;
            }
        }
        println!("Checkout contains {} entries", count);

        // show symlink status for library files
        if let Ok(entries) = std::fs::read_dir(checkout_path.join("usr/lib")) {
            println!("\n=== Library symlinks ===");
            for entry in entries.flatten().take(10) {
                let path = entry.path();
                let meta = path.symlink_metadata();
                if let Ok(m) = meta {
                    if m.file_type().is_symlink() {
                        let target = std::fs::read_link(&path).unwrap_or_default();
                        println!("  {} -> {}", path.file_name().unwrap().to_string_lossy(), target.display());
                    } else {
                        println!("  {} (file, {} bytes)", path.file_name().unwrap().to_string_lossy(), m.len());
                    }
                }
            }
        }
    }

    println!("\n=== Native OSTree implementation working! ===");
    Ok(())
}
