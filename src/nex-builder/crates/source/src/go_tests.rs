//! Tests for Go module and checksum parsing.

use std::fs;

use super::{escape, parse_go_mod, parse_go_sum, read_go_version};

#[test]
fn parses_requirements_sums_and_go_versions() {
    let required = parse_go_mod(
        "require one.example/a v1.0.0\nrequire (\n two.example/b v2.0.0 // indirect\n)\n",
    );
    assert_eq!(
        required.get("one.example/a").map(String::as_str),
        Some("v1.0.0")
    );
    assert_eq!(
        required.get("two.example/b").map(String::as_str),
        Some("v2.0.0")
    );
    let modules = parse_go_sum(
        "one.example/a v0.9.0 h1:old\none.example/a v1.0.0 h1:a\none.example/a v1.0.0/go.mod h1:b\n",
    );
    let selected = modules
        .iter()
        .filter(|module| required.get(&module.path) == Some(&module.version))
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].checksum, "h1:a");
    assert_eq!(modules.len(), 2);
    assert_eq!(escape("GitHub.com/A"), "!git!hub.com/!a");
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("go.mod"), "go 1.22 // comment\n").unwrap();
    assert_eq!(read_go_version(directory.path()).as_deref(), Some("1.22"));
}
