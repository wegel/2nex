//! Rules that inspect Rust source as text.

use crate::style::sources::StyleSource;
use crate::style::StyleViolation;

const FILE_SIZE_LIMIT: usize = 400;
const FILE_SIZE_EXCEPTION_WINDOW: usize = 20;

pub(super) fn check_text_rules(source: &StyleSource) -> Vec<StyleViolation> {
    let mut violations = Vec::new();
    let line_count = source.source.lines().count();

    if line_count > FILE_SIZE_LIMIT && !has_file_size_exception(&source.source) {
        violations.push(StyleViolation {
            rule_id: "STYLE-FILE-001",
            path: source.path.clone(),
            line: None,
            message: format!(
                "file has {line_count} lines and no STYLE-EXCEPTION(file-size): marker near the top"
            ),
        });
    }
    if !has_module_doc_comment(&source.source) {
        violations.push(StyleViolation {
            rule_id: "STYLE-DOC-001",
            path: source.path.clone(),
            line: Some(1),
            message: "file must start with //! after blank lines or crate attributes".to_owned(),
        });
    }
    violations
}

fn has_file_size_exception(source: &str) -> bool {
    source
        .lines()
        .take(FILE_SIZE_EXCEPTION_WINDOW)
        .any(|line| line.contains("STYLE-EXCEPTION(file-size):"))
}

fn has_module_doc_comment(source: &str) -> bool {
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("#![") {
            continue;
        }
        return trimmed.starts_with("//!");
    }
    false
}
