//! Overlay YAML application for system target roots.

use std::fs::{self, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::manifest::Overlay;

pub(super) fn apply_overlays(overlays: &[PathBuf], base_dir: &str) -> io::Result<()> {
    if overlays.is_empty() {
        return Ok(());
    }

    let target_dir = Path::new(base_dir).join("target");
    for overlay_path in overlays {
        apply_overlay_file(overlay_path, &target_dir)?;
    }
    Ok(())
}

fn apply_overlay_file(overlay_path: &Path, target_dir: &Path) -> io::Result<()> {
    if !overlay_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("overlay not found: {}", overlay_path.display()),
        ));
    }

    println!("  Applying overlay: {}", overlay_path.display());
    let overlay = read_overlay(overlay_path)?;
    let overlay_dir = overlay_path.parent().unwrap_or(Path::new("."));
    for entry in &overlay.files {
        apply_overlay_entry(entry, overlay_dir, target_dir)?;
    }
    Ok(())
}

fn read_overlay(overlay_path: &Path) -> io::Result<Overlay> {
    let overlay_content = fs::read_to_string(overlay_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("failed to read {}: {}", overlay_path.display(), e),
        )
    })?;
    serde_yaml::from_str(&overlay_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse {}: {}", overlay_path.display(), e),
        )
    })
}

fn apply_overlay_entry(
    entry: &crate::manifest::OverlayEntry,
    overlay_dir: &Path,
    target_dir: &Path,
) -> io::Result<()> {
    let rel_path = entry.path.strip_prefix("/").unwrap_or(&entry.path);
    let dst = target_dir.join(rel_path);
    remove_existing_destination(entry, &dst)?;
    create_parent_dir(&dst)?;
    write_overlay_entry(entry, overlay_dir, &dst)
}

fn remove_existing_destination(
    entry: &crate::manifest::OverlayEntry,
    dst: &Path,
) -> io::Result<()> {
    if dst.symlink_metadata().is_err() {
        return Ok(());
    }
    if dst.is_dir() && !dst.is_symlink() {
        return handle_existing_directory(entry, dst);
    }
    fs::remove_file(dst).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("failed to remove existing {}: {}", dst.display(), e),
        )
    })
}

fn handle_existing_directory(entry: &crate::manifest::OverlayEntry, dst: &Path) -> io::Result<()> {
    if entry.replace {
        return fs::remove_dir_all(dst).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("failed to remove dir {}: {}", dst.display(), e),
            )
        });
    }
    if entry.directory {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "overlay wants to replace directory {} with non-directory (use replace: true)",
            dst.display()
        ),
    ))
}

fn create_parent_dir(dst: &Path) -> io::Result<()> {
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                io::Error::new(
                    e.kind(),
                    format!("failed to create parent dir for {}: {}", dst.display(), e),
                )
            })?;
        }
    }
    Ok(())
}

fn write_overlay_entry(
    entry: &crate::manifest::OverlayEntry,
    overlay_dir: &Path,
    dst: &Path,
) -> io::Result<()> {
    if entry.directory {
        fs::create_dir_all(dst)?;
        return set_optional_mode(dst, entry.mode);
    }
    if let Some(target) = &entry.symlink {
        return create_overlay_symlink(dst, target);
    }
    if let Some(content) = &entry.content {
        return write_overlay_content(dst, content, entry.mode);
    }
    if let Some(source) = &entry.source {
        return copy_overlay_source(overlay_dir, source, dst, entry.mode);
    }
    fs::File::create(dst)?;
    set_optional_mode(dst, entry.mode)
}

fn create_overlay_symlink(dst: &Path, target: &Path) -> io::Result<()> {
    symlink(target, dst).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "failed to create symlink {} -> {}: {}",
                dst.display(),
                target.display(),
                e
            ),
        )
    })
}

fn write_overlay_content(dst: &Path, content: &str, mode: Option<u32>) -> io::Result<()> {
    let mut file = fs::File::create(dst).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("failed to create file {}: {}", dst.display(), e),
        )
    })?;
    file.write_all(content.as_bytes())?;
    set_optional_mode(dst, mode)
}

fn copy_overlay_source(
    overlay_dir: &Path,
    source: &Path,
    dst: &Path,
    mode: Option<u32>,
) -> io::Result<()> {
    let src = overlay_dir.join(source);
    fs::copy(&src, dst).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "failed to copy {} -> {}: {}",
                src.display(),
                dst.display(),
                e
            ),
        )
    })?;
    set_optional_mode(dst, mode)
}

fn set_optional_mode(path: &Path, mode: Option<u32>) -> io::Result<()> {
    if let Some(mode) = mode {
        fs::set_permissions(path, Permissions::from_mode(mode))?;
    }
    Ok(())
}
