use std::fs;
use std::path::Path;

use super::ManifestRepositories;

#[test]
fn conventional_product_loads_its_packages_before_upstream_nex() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let product = temp_dir.path().join("product");
    let upstream = product.join("upstream/nex");
    let manifest = product.join("asm/device.yaml");
    create_repository(&product);
    create_submodule_checkout(&upstream);
    fs::create_dir_all(product.join("pkg")).expect("product package dir");
    fs::create_dir_all(upstream.join("pkg")).expect("upstream package dir");
    fs::create_dir_all(manifest.parent().expect("manifest parent")).expect("assembly dir");
    fs::write(&manifest, "system: {}\n").expect("assembly manifest");

    let repositories = ManifestRepositories::discover(&manifest).expect("repositories");

    assert_eq!(repositories.product_root(), product.canonicalize().unwrap());
    assert_eq!(
        repositories.package_dirs(),
        vec![
            product.join("pkg").canonicalize().unwrap(),
            upstream.join("pkg").canonicalize().unwrap(),
        ]
    );
    assert!(repositories.is_writable(&manifest).expect("product owner"));
    assert!(!repositories
        .is_writable(&upstream.join("pkg"))
        .expect("upstream owner"));
}

#[test]
fn ordinary_nex_checkout_needs_no_upstream_directory() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let checkout = temp_dir.path().join("nex");
    create_repository(&checkout);
    fs::create_dir_all(checkout.join("pkg")).expect("package dir");

    let repositories = ManifestRepositories::discover(&checkout).expect("repositories");

    assert_eq!(repositories.package_dirs(), vec![checkout.join("pkg")]);
}

fn create_repository(path: &Path) {
    fs::create_dir_all(path.join(".git")).expect("git marker");
}

fn create_submodule_checkout(path: &Path) {
    fs::create_dir_all(path).expect("submodule directory");
    fs::write(
        path.join(".git"),
        "gitdir: ../../.git/modules/upstream/nex\n",
    )
    .expect("submodule git marker");
}

#[test]
fn a_plain_relative_reference_resolves_against_the_owning_repository() {
    let owning = std::path::Path::new("/product");
    let upstream = std::path::Path::new("/product/upstream/nex");

    let resolved = super::resolve_repository_reference(
        std::path::Path::new("asm/base.yaml"),
        owning,
        Some(upstream),
    );

    assert_eq!(resolved, std::path::Path::new("/product/asm/base.yaml"));
}

#[test]
fn a_nex_reference_resolves_against_the_upstream_repository() {
    let owning = std::path::Path::new("/product");
    let upstream = std::path::Path::new("/product/upstream/nex");

    let resolved = super::resolve_repository_reference(
        std::path::Path::new("nex:base/systemd.yaml"),
        owning,
        Some(upstream),
    );

    assert_eq!(
        resolved,
        std::path::Path::new("/product/upstream/nex/base/systemd.yaml"),
        "a nex: reference must not encode how deep the product sits"
    );
}

#[test]
fn a_nex_reference_falls_back_to_the_owning_repository() {
    let owning = std::path::Path::new("/nex");

    let resolved =
        super::resolve_repository_reference(std::path::Path::new("nex:base/systemd.yaml"), owning, None);

    assert_eq!(
        resolved,
        std::path::Path::new("/nex/base/systemd.yaml"),
        "building inside Nex itself makes the owning repository the Nex repository"
    );
}

#[test]
fn an_absolute_reference_is_left_alone() {
    let resolved = super::resolve_repository_reference(
        std::path::Path::new("/elsewhere/base.yaml"),
        std::path::Path::new("/product"),
        None,
    );

    assert_eq!(resolved, std::path::Path::new("/elsewhere/base.yaml"));
}
