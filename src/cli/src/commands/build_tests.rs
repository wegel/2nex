use std::fs;
use std::path::Path;

use super::BuildOpts;

#[test]
fn build_write_flags_reject_an_imported_manifest() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let product = temp_dir.path().join("product");
    let upstream = product.join("upstream/nex");
    let manifest = upstream.join("pkg/demo.yaml");
    create_repository(&product);
    create_repository(&upstream);
    fs::create_dir_all(manifest.parent().unwrap()).expect("package dir");
    fs::write(&manifest, "package: {}\n").expect("manifest");
    let opts = opts(&manifest, &product, true);

    let error = opts
        .ensure_manifest_write_allowed()
        .expect_err("imported manifest write should fail");

    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(error.to_string().contains("imported manifest"));
}

#[test]
fn read_only_build_flags_allow_an_imported_manifest() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let product = temp_dir.path().join("product");
    let upstream = product.join("upstream/nex");
    let manifest = upstream.join("pkg/demo.yaml");
    create_repository(&product);
    create_repository(&upstream);
    fs::create_dir_all(manifest.parent().unwrap()).expect("package dir");
    fs::write(&manifest, "package: {}\n").expect("manifest");
    let opts = opts(&manifest, &product, false);

    opts.ensure_manifest_write_allowed()
        .expect("read-only build should be allowed");
}

#[test]
fn orchestrated_imported_build_drops_write_flags() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let product = temp_dir.path().join("product");
    let upstream = product.join("upstream/nex");
    let manifest = upstream.join("pkg/demo.yaml");
    create_repository(&product);
    create_repository(&upstream);
    fs::create_dir_all(manifest.parent().unwrap()).expect("package dir");
    fs::write(&manifest, "package: {}\n").expect("manifest");
    let mut opts = opts(&manifest, &product, true);
    opts.compute_deps = true;
    opts.refresh_metadata = true;
    opts.generate_outputs = true;
    opts.record_profile = true;
    opts.add_checksums = true;

    opts.restrict_imported_manifest_writes()
        .expect("imported build should become read-only");

    assert!(!opts.update_checksum);
    assert!(!opts.compute_deps);
    assert!(!opts.refresh_metadata);
    assert!(!opts.generate_outputs);
    assert!(!opts.record_profile);
    assert!(!opts.add_checksums);
    opts.ensure_manifest_write_allowed()
        .expect("restricted imported build should be safe");
}

fn create_repository(path: &Path) {
    fs::create_dir_all(path.join(".git")).expect("git marker");
}

fn opts(manifest: &Path, product: &Path, update_checksum: bool) -> BuildOpts {
    BuildOpts {
        repo_path: ".nex/repo".to_string(),
        manifest_file: manifest.to_string_lossy().into_owned(),
        manifest_dirs: Vec::new(),
        writable_manifest_root: Some(product.canonicalize().expect("product root")),
        check: false,
        update_checksum,
        compute_deps: false,
        runtime_deps_verbose: false,
        refresh_metadata: false,
        force: false,
        build_dir: None,
        generate_outputs: false,
        fallback_repos: Vec::new(),
        verbose: false,
        record_profile: false,
        no_progress: true,
        dry_run: false,
        add_checksums: false,
        show_dep_paths: false,
        trace_dependency: None,
        multi_progress: None,
        reuse_rootfs: false,
    }
}
