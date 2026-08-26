//! Commit-message checks used by the tracked hook.

use anyhow::{bail, Result};

pub(crate) fn validate_commit_message(raw: &str) -> Result<()> {
    let lines = non_comment_lines(raw);
    if lines.len() != 1 {
        bail!(failure_message());
    }

    let subject = lines[0];
    if subject.ends_with('.') {
        bail!(failure_message());
    }

    let Some((scope, summary)) = subject.split_once(": ") else {
        bail!(failure_message());
    };
    if !valid_scope(scope) || summary.trim().is_empty() {
        bail!(failure_message());
    }

    Ok(())
}

pub(crate) fn failure_message() -> &'static str {
    r#"commit message must match "<scope>: <subject>" and must not end with a period"#
}

fn non_comment_lines(raw: &str) -> Vec<&str> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn valid_scope(scope: &str) -> bool {
    !scope.is_empty()
        && scope.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

#[cfg(test)]
mod tests {
    use super::validate_commit_message;

    #[test]
    fn accepts_scoped_subject() {
        assert!(validate_commit_message("builder: cache verified sources\n").is_ok());
    }

    #[test]
    fn accepts_git_comments() {
        assert!(validate_commit_message("repo: add hooks\n\n# staged files follow\n").is_ok());
    }

    #[test]
    fn rejects_unscoped_subject() {
        assert!(validate_commit_message("add hooks\n").is_err());
    }

    #[test]
    fn rejects_uppercase_scope() {
        assert!(validate_commit_message("Repo: add hooks\n").is_err());
    }

    #[test]
    fn rejects_body() {
        assert!(validate_commit_message("repo: add hooks\n\nMore detail\n").is_err());
    }

    #[test]
    fn rejects_period() {
        assert!(validate_commit_message("repo: add hooks.\n").is_err());
    }
}
