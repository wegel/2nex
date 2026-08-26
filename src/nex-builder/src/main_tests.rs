//! Tests for command-line output modes.

use clap::Parser;

use super::Args;

#[test]
fn verbose_and_quiet_conflict() {
    assert!(Args::try_parse_from(["nex-builder", "manifest.yaml", "-v", "-q"]).is_err());
}

#[test]
fn accepts_plain_verbose_output() {
    let args = Args::try_parse_from(["nex-builder", "manifest.yaml", "--verbose", "--no-progress"])
        .expect("parse verbose output flags");
    assert!(args.verbose);
    assert!(args.no_progress);
}

#[test]
fn rejects_zero_resource_limits() {
    assert!(Args::try_parse_from(["nex-builder", "manifest.yaml", "--jobs", "0"]).is_err());
    assert!(Args::try_parse_from(["nex-builder", "manifest.yaml", "--cpus", "0"]).is_err());
}

#[test]
fn accepts_explicit_adoption_of_existing_refs() {
    let args = Args::try_parse_from(["nex-builder", "manifest.yaml", "--adopt-existing"])
        .expect("parse adoption flag");
    assert!(args.adopt_existing);
}
