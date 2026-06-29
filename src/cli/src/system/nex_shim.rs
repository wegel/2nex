//! Nex dynamic loader shim installation.

use std::io;
use std::path::Path;

use crate::store::{export_path, Store};

pub(super) fn install_nex_ld_shim(repo_path: &str, lib64_dir: &Path) -> io::Result<()> {
    let shim_path = lib64_dir.join("ld-linux-x86-64.so.2");
    let shim_ref = find_nex_ld_shim_ref(repo_path)?;
    println!("Using packaged nex-ld-shim: {}", shim_ref);

    export_path(
        repo_path,
        &shim_ref,
        "/usr/lib/nex-ld-shim",
        &shim_path,
        false,
    )?;
    println!("  Installed nex-ld-shim at /lib64/ld-linux-x86-64.so.2");
    Ok(())
}

fn find_nex_ld_shim_ref(repo_path: &str) -> io::Result<String> {
    let store = Store::open(repo_path)?;
    let refs = store.refs(None)?;
    refs.iter()
        .find(|entry| entry.contains("nex-ld-shim") && entry.contains("/deploy/"))
        .or_else(|| {
            refs.iter()
                .find(|entry| entry.contains("nex-ld-shim") && entry.contains("/bundles/"))
        })
        .or_else(|| {
            refs.iter()
                .find(|entry| entry.contains("nex-ld-shim") && entry.contains("/outputs/"))
        })
        .cloned()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "nex-ld-shim not found. Build with: nex build <repo> pkg/core/nex-ld-shim/nex-ld-shim.yaml",
            )
        })
}
