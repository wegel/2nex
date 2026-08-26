//! Tests for Rust source rules.

use crate::style::sources::StyleSource;
use crate::style::{rust, CrateKind, StyleViolation};

fn check_source(path: &str, source: &str, crate_kind: CrateKind) -> Vec<StyleViolation> {
    rust::check_rust_source(&StyleSource {
        path: path.into(),
        crate_kind,
        source: source.to_owned(),
    })
}

#[test]
fn flags_glob_imports() {
    let violations = check_source(
        "src/example/src/lib.rs",
        "//! Example\nuse super::*;\n",
        CrateKind::Library,
    );
    assert!(violations
        .iter()
        .any(|violation| violation.rule_id == "STYLE-IMP-001"));
}

#[test]
fn requires_file_size_exception_marker() {
    let body = (0..401)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect::<String>();
    let violations = check_source(
        "src/example/src/main.rs",
        &format!("//! Example\n{body}"),
        CrateKind::Binary,
    );
    assert!(violations
        .iter()
        .any(|violation| violation.rule_id == "STYLE-FILE-001"));
}

#[test]
fn accepts_file_size_exception_marker() {
    let body = (0..401)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect::<String>();
    let violations = check_source(
        "src/example/src/main.rs",
        &format!("//! Example\n// STYLE-EXCEPTION(file-size): commands stay together\n{body}"),
        CrateKind::Binary,
    );
    assert!(!violations
        .iter()
        .any(|violation| violation.rule_id == "STYLE-FILE-001"));
}

#[test]
fn flags_long_functions() {
    let body = (0..61)
        .map(|index| format!("    let value_{index} = {index};\n"))
        .collect::<String>();
    let violations = check_source(
        "src/example/src/main.rs",
        &format!("//! Example\nfn too_long() {{\n{body}}}\n"),
        CrateKind::Binary,
    );
    assert!(violations
        .iter()
        .any(|violation| violation.rule_id == "STYLE-FUNC-001"));
}

#[test]
fn flags_inline_tests_in_large_files() {
    let filler = (0..100)
        .map(|index| format!("fn filler_{index}() {{}}\n"))
        .collect::<String>();
    let violations = check_source(
        "src/example/src/main.rs",
        &format!("//! Example\n{filler}\n#[cfg(test)]\nmod tests {{}}\n"),
        CrateKind::Binary,
    );
    assert!(violations
        .iter()
        .any(|violation| violation.rule_id == "STYLE-TEST-001"));
}

#[test]
fn flags_public_anyhow_returns() {
    let violations = check_source(
        "src/example/src/lib.rs",
        "//! Example\nuse anyhow::Result;\npub fn bad() -> Result<()> { Ok(()) }\n",
        CrateKind::Library,
    );
    assert!(violations
        .iter()
        .any(|violation| violation.rule_id == "STYLE-ERR-001"));
}

#[test]
fn flags_public_error_enum_anyhow_variant() {
    let violations = check_source(
        "src/example/src/lib.rs",
        "//! Example\npub enum ExampleError { Anyhow(anyhow::Error) }\n",
        CrateKind::Library,
    );
    assert!(violations
        .iter()
        .any(|violation| violation.rule_id == "STYLE-ERR-002"));
}

#[test]
fn requires_module_doc_comment() {
    let violations = check_source(
        "src/example/src/main.rs",
        "fn main() {}\n",
        CrateKind::Binary,
    );
    assert!(violations
        .iter()
        .any(|violation| violation.rule_id == "STYLE-DOC-001"));
}
