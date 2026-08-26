//! Parse one Rust file and apply every source rule.

mod ast;
mod text;

use crate::style::sources::StyleSource;
use crate::style::StyleViolation;

pub(crate) fn check_rust_source(source: &StyleSource) -> Vec<StyleViolation> {
    let mut violations = text::check_text_rules(source);
    let Ok(parsed) = syn::parse_file(&source.source) else {
        violations.push(StyleViolation {
            rule_id: "STYLE-PARSE-001",
            path: source.path.clone(),
            line: None,
            message: "could not parse Rust source".to_owned(),
        });
        return violations;
    };

    ast::inspect_items(
        &parsed,
        &source.path,
        source.crate_kind,
        source.source.lines().count(),
        &mut violations,
    );
    violations
}
