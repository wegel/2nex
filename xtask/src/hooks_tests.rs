//! Tests for Cargo command construction.

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::cargo_args;
use crate::cargo_project::{CargoScope, RustProject};
use crate::config::HookPhase;

fn project() -> RustProject {
    RustProject {
        name: "bootloader".to_owned(),
        manifest_path: PathBuf::from("src/bootloader/Cargo.toml"),
        root: PathBuf::from("src/bootloader"),
        packages: Vec::new(),
        clippy_args: vec!["--target".to_owned(), "host".to_owned()],
        test_args: vec!["--no-default-features".to_owned()],
    }
}

#[test]
fn builds_workspace_clippy_command() {
    assert_eq!(
        cargo_args(HookPhase::Clippy, &project(), &CargoScope::Workspace),
        [
            "clippy",
            "--manifest-path",
            "src/bootloader/Cargo.toml",
            "--workspace",
            "--target",
            "host",
            "--",
            "-D",
            "warnings",
        ]
    );
}

#[test]
fn builds_package_test_command() {
    let scope = CargoScope::Packages(BTreeSet::from(["nex-bootloader".to_owned()]));
    assert_eq!(
        cargo_args(HookPhase::Test, &project(), &scope),
        [
            "test",
            "--manifest-path",
            "src/bootloader/Cargo.toml",
            "--package",
            "nex-bootloader",
            "--no-default-features",
        ]
    );
}
