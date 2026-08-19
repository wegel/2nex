//! Assembly file entry application for system target roots.

use std::fs::{self, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::manifest::AssemblyFile;

pub(super) fn apply_file_entries(entries: &[AssemblyFile], base_dir: &str) -> io::Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    println!("  Applying {} assembly file entries", entries.len());
    let target_dir = Path::new(base_dir).join("target");
    for entry in entries {
        apply_file_entry(entry, &target_dir)?;
    }
    Ok(())
}

fn apply_file_entry(entry: &AssemblyFile, target_dir: &Path) -> io::Result<()> {
    let rel_path = entry.path.strip_prefix("/").unwrap_or(&entry.path);
    let dst = target_dir.join(rel_path);
    remove_existing_destination(entry, &dst)?;
    create_parent_dir(&dst)?;
    write_file_entry(entry, &dst)
}

fn remove_existing_destination(entry: &AssemblyFile, dst: &Path) -> io::Result<()> {
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

fn handle_existing_directory(entry: &AssemblyFile, dst: &Path) -> io::Result<()> {
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
            "assembly file entry wants to replace directory {} with non-directory (use replace: true)",
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

fn write_file_entry(entry: &AssemblyFile, dst: &Path) -> io::Result<()> {
    if entry.directory {
        fs::create_dir_all(dst)?;
        return set_optional_mode(dst, entry.mode);
    }
    if let Some(target) = &entry.symlink {
        return create_file_symlink(dst, target);
    }
    if let Some(content) = &entry.content {
        return write_file_content(dst, content, entry.mode);
    }
    if let Some(source) = &entry.source {
        let base_dir = entry.base_dir.clone().unwrap_or_else(|| PathBuf::from("."));
        let src = crate::manifest::resolve_repository_reference(
            source,
            &base_dir,
            entry.upstream_dir.as_deref(),
        );
        return copy_file_source(&src, dst, entry.mode);
    }
    fs::File::create(dst)?;
    set_optional_mode(dst, entry.mode)
}

fn create_file_symlink(dst: &Path, target: &Path) -> io::Result<()> {
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

fn write_file_content(dst: &Path, content: &str, mode: Option<u32>) -> io::Result<()> {
    let mut file = fs::File::create(dst).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("failed to create file {}: {}", dst.display(), e),
        )
    })?;
    file.write_all(content.as_bytes())?;
    set_optional_mode(dst, mode)
}

fn copy_file_source(src: &Path, dst: &Path, mode: Option<u32>) -> io::Result<()> {
    fs::copy(src, dst).map_err(|e| {
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
