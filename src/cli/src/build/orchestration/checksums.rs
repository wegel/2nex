//! Missing-checksum repair for dependency build plans.

use std::io;

use petgraph::graph::{DiGraph, NodeIndex};

use crate::commands::build::BuildOpts;
use crate::manifest::types::{Manifest, ManifestSource};
use crate::manifest::update::update_manifest_checksum_field;
use crate::manifest::{load_manifest_from_source, ManifestData, ManifestKind};
use crate::store::get_branch_metadata;

use super::manifest_lookup::get_build_dir_for_package;

/// Build or inspect packages in the plan so manifests gain missing checksums.
pub fn add_missing_checksums_to_manifests(
    build_order: &[NodeIndex],
    graph: &DiGraph<ManifestSource, ()>,
    repo_path: &str,
    opts: &BuildOpts,
) -> io::Result<()> {
    for &node_idx in build_order {
        let source = &graph[node_idx];
        let manifest_data = load_manifest_from_source(source)?;
        if let ManifestData::Package(manifest) = manifest_data {
            add_package_checksum_if_missing(source, repo_path, opts, &manifest)?;
        }
    }

    Ok(())
}

fn add_package_checksum_if_missing(
    source: &ManifestSource,
    repo_path: &str,
    opts: &BuildOpts,
    manifest: &Manifest,
) -> io::Result<()> {
    if manifest.package.checksum.is_some() {
        return Ok(());
    }

    println!("Processing: {}", manifest.package.slug);
    if update_from_store_checksum(source, repo_path, manifest)? {
        return Ok(());
    }

    build_package_for_checksum(source, repo_path, opts, manifest)?;
    println!("  Built and checksummed");
    Ok(())
}

fn update_from_store_checksum(
    source: &ManifestSource,
    repo_path: &str,
    manifest: &Manifest,
) -> io::Result<bool> {
    let Some((bundle_name, _)) = manifest.bundles.iter().next() else {
        return Ok(false);
    };
    let commit_ref = format!(
        "x86_64/{}/{}/{}/bundles/{}",
        manifest.package.namespace_path(),
        manifest.package.slug,
        manifest.package.version,
        bundle_name
    );

    match get_branch_metadata(repo_path, &commit_ref, "nex.output.checksum") {
        Ok(checksum) => {
            println!("  Found checksum in store: {}", checksum);
            update_manifest_checksum_field(
                source.path().to_str().unwrap(),
                ManifestKind::Package,
                &checksum,
            )?;
            Ok(true)
        }
        Err(_) => {
            println!("  Not found in store, building to get checksum...");
            Ok(false)
        }
    }
}

fn build_package_for_checksum(
    source: &ManifestSource,
    repo_path: &str,
    opts: &BuildOpts,
    manifest: &Manifest,
) -> io::Result<()> {
    let mut manifest_copy = manifest.clone();
    let build_dir = get_build_dir_for_package(manifest);
    let build_opts = checksum_build_opts(source, repo_path, opts);
    crate::build::build_package_manifest_with_dir(&build_opts, &mut manifest_copy, &build_dir)
}

fn checksum_build_opts(source: &ManifestSource, repo_path: &str, opts: &BuildOpts) -> BuildOpts {
    BuildOpts {
        repo_path: repo_path.to_string(),
        manifest_file: source.path().to_string_lossy().to_string(),
        check: false,
        update_checksum: true,
        compute_deps: opts.compute_deps,
        runtime_deps_verbose: opts.runtime_deps_verbose,
        refresh_metadata: false,
        force: false,
        build_dir: None,
        generate_outputs: false,
        fallback_repos: opts.fallback_repos.clone(),
        verbose: opts.verbose,
        record_profile: opts.record_profile,
        no_progress: opts.no_progress,
        dry_run: false,
        add_checksums: false,
        show_dep_paths: false,
        trace_dependency: None,
        multi_progress: None,
        reuse_rootfs: false,
    }
}
