//! Tests for Zig dependency parsing.

use super::parse_zon;

#[test]
fn parses_comments_and_git_dependencies() {
    let zon = r#"
        .dependencies = .{
            // comment with https://ignored
            .ghostty = .{
                .url = "git+https://github.com/ghostty-org/ghostty.git?ref=HEAD#abcdef",
                .hash = "ghostty-1.0-Ab_C",
            },
        },
    "#;
    let dependencies = parse_zon(zon).unwrap();
    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0].name, "ghostty");
    assert_eq!(
        dependencies[0].url,
        "https://github.com/ghostty-org/ghostty"
    );
    assert_eq!(dependencies[0].revision.as_deref(), Some("abcdef"));
}
