//! Shared helpers for copying relative symlink targets safely.

use std::io;
use std::path::{Component, Path, PathBuf};

pub(super) const MAX_RELATIVE_SYMLINK_HOPS: usize = 40;

pub(super) fn reject_too_many_symlink_hops(src: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("too many symlink hops while copying {}", src.display()),
    ))
}

pub(super) fn relative_symlink_target_inside_root(
    root: &Path,
    src: &Path,
    link_target: &Path,
) -> io::Result<PathBuf> {
    let Some(parent) = src.parent() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("symlink {} has no parent directory", src.display()),
        ));
    };

    let target = normalize_path(&parent.join(link_target));
    if !target.starts_with(root) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "relative symlink target escapes checkout root: {} -> {}",
                src.display(),
                link_target.display()
            ),
        ));
    }

    target
        .strip_prefix(root)
        .map(Path::to_path_buf)
        .map_err(io::Error::other)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
        }
    }
    normalized
}
