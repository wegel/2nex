use std::fs;
use std::sync::Arc;

use indicatif::MultiProgress;

use super::build_opts_for_node;
use crate::commands::build::BuildOpts;

#[test]
fn imported_dependency_nodes_build_without_manifest_write_flags() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let product = temp_dir.path().join("product");
    let upstream = product.join("upstream/nex");
    let manifest = upstream.join("pkg/demo.yaml");
    fs::create_dir_all(product.join(".git")).expect("product git marker");
    fs::create_dir_all(upstream.join(".git")).expect("upstream git marker");
    fs::create_dir_all(manifest.parent().unwrap()).expect("package directory");
    fs::write(&manifest, "package: {}\n").expect("package manifest");
    let opts = writable_opts(&product);

    let node_opts = build_opts_for_node(&opts, &manifest, Arc::new(MultiProgress::new()))
        .expect("node options");

    assert!(node_opts.check);
    assert!(node_opts.force);
    assert!(!node_opts.update_checksum);
    assert!(!node_opts.compute_deps);
    assert!(!node_opts.refresh_metadata);
    assert!(!node_opts.generate_outputs);
    assert!(!node_opts.record_profile);
    assert!(!node_opts.add_checksums);
}

fn writable_opts(product: &std::path::Path) -> BuildOpts {
    BuildOpts {
        repo_path: ".nex/repo".to_string(),
        manifest_file: product
            .join("asm/device.yaml")
            .to_string_lossy()
            .into_owned(),
        manifest_dirs: Vec::new(),
        writable_manifest_root: Some(product.canonicalize().expect("product root")),
        check: true,
        update_checksum: true,
        compute_deps: true,
        runtime_deps_verbose: false,
        refresh_metadata: true,
        force: true,
        build_dir: None,
        generate_outputs: true,
        fallback_repos: Vec::new(),
        verbose: false,
        record_profile: true,
        no_progress: true,
        dry_run: false,
        add_checksums: true,
        show_dep_paths: false,
        trace_dependency: None,
        multi_progress: None,
        reuse_rootfs: false,
    }
}
