//! Tests for Cargo lockfile parsing.

use super::parse_lock;

#[test]
fn parses_registry_git_and_legacy_packages() {
    let lock = r#"
[[package]]
name = "one"
version = "1.0.0"
source = "registry+https://example.invalid/index"
checksum = "1111"

[[package]]
name = "two"
version = "2.0.0"
source = "git+https://example.invalid/two?rev=main#abcdef"

[[package]]
name = "three"
version = "3.0.0"
source = "registry+https://example.invalid/index"

[metadata]
"checksum three 3.0.0 (registry+https://example.invalid/index)" = "3333"
"#;
    let (registry, git) = parse_lock(lock).unwrap();
    assert_eq!(registry.len(), 2);
    assert_eq!(registry[1].checksum, "3333");
    assert_eq!(git.len(), 1);
    assert_eq!(git[0].url, "https://example.invalid/two");
    assert_eq!(git[0].commit, "abcdef");
}
