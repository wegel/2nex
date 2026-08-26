//! Tests for capsule merging and public links.

use std::fs;
use std::path::PathBuf;

use zub::Repo;

use super::{deploy_catalog, ensure_capsule_loader, install_roots, materialize, merge_missing};
use crate::assembly_manifest::PackagePlacement;
use crate::graph::PackagePlan;
use crate::manifest::Dependency;
use crate::nex_links::link_capsule;
use crate::reference::InputRef;

#[test]
fn runtime_files_do_not_replace_root_files() {
    let temporary = tempfile::tempdir().expect("create capsule fixture directory");
    let source = temporary.path().join("runtime");
    let capsule = temporary.path().join("capsule");
    fs::create_dir_all(source.join("usr/lib")).expect("create runtime tree");
    fs::create_dir_all(capsule.join("usr/lib")).expect("create capsule tree");
    fs::write(source.join("usr/lib/shared"), "runtime").expect("write shared runtime file");
    fs::write(source.join("usr/lib/added"), "added").expect("write added runtime file");
    fs::write(capsule.join("usr/lib/shared"), "root").expect("write capsule-owned file");

    merge_missing(&source, &capsule).expect("merge runtime into capsule");

    assert_eq!(
        fs::read_to_string(capsule.join("usr/lib/shared")).expect("read capsule-owned file"),
        "root"
    );
    assert_eq!(
        fs::read_to_string(capsule.join("usr/lib/added")).expect("read merged runtime file"),
        "added"
    );
}

#[test]
fn package_files_link_to_the_logical_capsule() {
    let temporary = tempfile::tempdir().expect("create link fixture directory");
    let path = temporary.path().join("physical");
    let target = temporary.path().join("target");
    fs::create_dir_all(path.join("usr/bin")).expect("create capsule binary directory");
    fs::create_dir(&target).expect("create system target");
    fs::write(path.join("usr/bin/tool"), "tool").expect("write capsule tool");
    let logical = PathBuf::from("/nex/pkg/core/tool/1/01234567");

    link_capsule(&path, &logical, &target).expect("link capsule into system");

    assert_eq!(
        fs::read_link(target.join("usr/bin/tool")).expect("read public tool link"),
        PathBuf::from("/nex/pkg/core/tool/1/01234567/usr/bin/tool")
    );
}

#[test]
fn capsule_loader_points_to_its_private_glibc() {
    let temporary = tempfile::tempdir().expect("create loader fixture directory");
    let capsule = temporary.path();
    fs::create_dir_all(capsule.join("usr/lib")).expect("create capsule library directory");
    fs::write(capsule.join("usr/lib/ld-linux-x86-64.so.2"), "loader")
        .expect("write capsule loader");

    ensure_capsule_loader(capsule).expect("link capsule loader");

    assert_eq!(
        fs::read_link(capsule.join("lib/ld-linux-x86-64.so.2")).expect("read capsule loader link"),
        PathBuf::from("../usr/lib/ld-linux-x86-64.so.2")
    );
}

#[test]
fn deployed_catalog_contains_only_manifests() {
    let temporary = tempfile::tempdir().expect("create catalog fixture directory");
    let catalog = temporary.path().join("catalog");
    let target = temporary.path().join("target");
    fs::create_dir_all(catalog.join("pkg/core")).expect("create catalog package directory");
    fs::create_dir(&target).expect("create catalog target");
    fs::write(catalog.join("pkg/core/tool.yaml"), "package: {}\n").expect("write catalog manifest");
    fs::write(catalog.join("pkg/core/source.c"), "int main(void) {}\n")
        .expect("write non-manifest catalog file");

    deploy_catalog(&catalog, &target).expect("deploy catalog manifests");

    assert!(target.join("nex/db/pkg/core/tool.yaml").is_file());
    assert!(!target.join("nex/db/pkg/core/source.c").exists());
}

#[test]
fn materializes_a_complete_capsule_and_public_links() {
    let fixture = MaterializeFixture::new();

    materialize(
        &fixture.repo,
        &[fixture.plan],
        &fixture.catalog,
        &fixture.build,
        &fixture.target,
    )
    .expect("materialize complete capsule");

    let capsule = fixture.target.join("nex/pkg/core/tool/1/01234567");
    assert!(capsule.join("usr/bin/tool").is_file());
    assert!(capsule.join("usr/lib/libc.so.6").is_file());
    assert_eq!(
        fs::read_link(capsule.join("lib/ld-linux-x86-64.so.2")).expect("read private loader link"),
        PathBuf::from("../usr/lib/ld-linux-x86-64.so.2")
    );
    assert_eq!(
        fs::read_link(fixture.target.join("usr/bin/tool")).expect("read public tool link"),
        PathBuf::from("/nex/pkg/core/tool/1/01234567/usr/bin/tool")
    );
    assert_eq!(
        fs::read_link(fixture.target.join("usr/lib/libc.so.6")).expect("read public library link"),
        PathBuf::from("../../nex/pkg/core/tool/1/01234567/usr/lib/libc.so.6")
    );
    assert_eq!(
        fs::read(fixture.target.join("lib64/ld-linux-x86-64.so.2"))
            .expect("read installed loader shim"),
        b"shim"
    );
}

#[test]
fn explicit_root_placement_bypasses_the_capsule() {
    let temporary = tempfile::tempdir().expect("create root-placement fixture directory");
    let repo = Repo::init(&temporary.path().join("repo")).expect("initialize placement repo");
    let source = temporary.path().join("source/usr/lib/modules");
    let target = temporary.path().join("target");
    let package_root = target.join("nex/pkg");
    fs::create_dir_all(&source).expect("create module source tree");
    fs::create_dir(&target).expect("create system target");
    fs::write(source.join("module.ko"), "module").expect("write test module");
    let reference = "x86_64/pkg/tests/tool/1/outputs/modules";
    zub::ops::commit(
        &repo,
        &temporary.path().join("source"),
        reference,
        None,
        Some("test"),
    )
    .expect("commit root-placed package");
    let plan = PackagePlan {
        root: dependency(reference),
        runtime: Vec::new(),
        placement: PackagePlacement::Root,
    };

    let plans = [plan];
    let capsules =
        install_roots(&repo, &plans, &package_root, &target).expect("install root-placed package");

    assert!(capsules.is_empty());
    assert_eq!(
        fs::read(target.join("usr/lib/modules/module.ko")).expect("read placed module"),
        b"module"
    );
    assert!(!package_root.exists());
}

struct MaterializeFixture {
    _temporary: tempfile::TempDir,
    repo: Repo,
    catalog: PathBuf,
    build: PathBuf,
    target: PathBuf,
    plan: PackagePlan,
}

impl MaterializeFixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("create materialization fixture directory");
        let repo = Repo::init(&temporary.path().join("repo")).expect("initialize capsule repo");
        let root = temporary.path().join("root/usr/bin");
        let runtime = temporary.path().join("runtime/usr/lib");
        let catalog = temporary.path().join("catalog");
        let build = temporary.path().join("build");
        let target = temporary.path().join("target");
        fs::create_dir_all(&root).expect("create capsule root");
        fs::create_dir_all(&runtime).expect("create capsule runtime");
        fs::create_dir_all(catalog.join("pkg/core")).expect("create fixture catalog");
        fs::create_dir_all(build.join("usr/lib")).expect("create build files");
        fs::create_dir(&target).expect("create system target");
        fs::write(root.join("tool"), "tool").expect("write package tool");
        fs::write(runtime.join("libc.so.6"), "libc").expect("write runtime library");
        fs::write(runtime.join("ld-linux-x86-64.so.2"), "loader").expect("write runtime loader");
        fs::write(catalog.join("pkg/core/tool.yaml"), "package: {}\n")
            .expect("write embedded manifest");
        fs::write(build.join("usr/lib/nex-ld-shim"), "shim").expect("write loader shim");
        let package_ref = "x86_64/pkg/core/tool/1/outputs/bin";
        zub::ops::commit_with_metadata(
            &repo,
            &temporary.path().join("root"),
            package_ref,
            None,
            Some("test"),
            &[("nex.build.checksum", CHECKSUM)],
        )
        .expect("commit package root");
        let runtime_commit = zub::ops::commit(
            &repo,
            &temporary.path().join("runtime"),
            "runtime",
            None,
            Some("test"),
        )
        .expect("commit package runtime");
        let plan = PackagePlan {
            root: dependency(package_ref),
            runtime: vec![dependency(&runtime_commit.to_string())],
            placement: PackagePlacement::Capsule,
        };
        Self {
            _temporary: temporary,
            repo,
            catalog,
            build,
            target,
            plan,
        }
    }
}

const CHECKSUM: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn dependency(commit: &str) -> Dependency {
    Dependency {
        name: None,
        commit: InputRef::parse(commit).expect("valid test dependency ref"),
        paths: Vec::new(),
    }
}
