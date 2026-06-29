//! Relative path calculation for materializer symlinks.

use std::path::{Component, Path, PathBuf};

/// Calculate the relative path from `base` to `target`.
pub(super) fn diff_paths(target: &Path, base: &Path) -> Option<PathBuf> {
    let mut target_components = target.components().peekable();
    let mut base_components = base.components().peekable();
    while target_components.peek() == base_components.peek() {
        target_components.next();
        if base_components.next().is_none() {
            break;
        }
    }

    let ups = base_components
        .filter(|component| matches!(component, Component::Normal(_)))
        .count();

    let mut result = PathBuf::new();
    for _ in 0..ups {
        result.push("..");
    }
    for component in target_components {
        result.push(component);
    }

    Some(result)
}
