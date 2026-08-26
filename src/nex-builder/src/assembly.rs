//! System assembly execution and publication.

use std::fs;
use std::io;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use zub::Repo;

use crate::assembly_manifest::{AssemblyFile, AssemblyFileKind, AssemblyManifest};
use crate::builder::{
    checkout_dependencies, checkout_reference, reset_root, verify_reproducible, BuildContext,
};
use crate::catalog;
use crate::checksum::output_checksum;
use crate::graph::PackageLayout;
use crate::manifest::Dependency;
use crate::metadata;
use crate::progress::BuildPass;
use crate::source::stage_sources;
use crate::{factory, nex_layout, sandbox, ChecksumKind, Error, Result};

static ASSEMBLY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn run_assembly(
    manifest: &AssemblyManifest,
    build_inputs: &[Dependency],
    packages: &PackageLayout,
    context: &BuildContext<'_>,
) -> Result<()> {
    let checksum = build_once(
        manifest,
        build_inputs,
        packages,
        context,
        context.build_root,
        BuildPass::Primary,
    )?;
    verify_checksum(manifest, &checksum)?;
    verify_reproducible(&checksum, context, |root, pass| {
        build_once(manifest, build_inputs, packages, context, root, pass)
    })?;
    let reference = manifest.reference();
    publish(
        context.repo,
        &context.build_root.join("target"),
        &reference,
        &checksum,
        context.recipe,
    )
    .map_err(|source| Error::Publication { reference, source })
}

fn build_once(
    manifest: &AssemblyManifest,
    build_inputs: &[Dependency],
    packages: &PackageLayout,
    context: &BuildContext<'_>,
    root: &Path,
    pass: BuildPass,
) -> io::Result<String> {
    reset_root(root, context.environment)?;
    let root = root.canonicalize()?;
    let target = root.join("target");
    fs::create_dir(&target)?;
    checkout_dependencies(context.repo, build_inputs, &root)?;
    if let Some(base) = &manifest.base {
        checkout_reference(context.repo, &base.commit, &target)?;
    }
    let base_defaults = match (packages, manifest.base.as_ref()) {
        (PackageLayout::Nex(_), Some(_)) => Some(factory::capture(&target)?),
        _ => None,
    };
    match packages {
        PackageLayout::Flat(inputs) => checkout_dependencies(context.repo, inputs, &target)?,
        PackageLayout::Nex(capsules) => nex_layout::materialize(
            context.repo,
            capsules,
            &catalog::root(context.manifest_path)?,
            &root,
            &target,
        )?,
    }
    materialize_files(&manifest.files, context.manifest_path, &target)?;
    let sources = stage_sources(
        &manifest.sources,
        context.manifest_path,
        context.source_cache,
        &root,
        context.environment,
    )?;
    sandbox::run(
        &root,
        &manifest.build.script,
        context.environment,
        &sources,
        context.cpu_count,
        context.output,
        pass,
    )?;
    if matches!(packages, PackageLayout::Nex(_)) {
        factory::move_etc(&target, base_defaults.as_ref())?;
    }
    output_checksum(&target)
}

fn materialize_files(files: &[AssemblyFile], manifest: &Path, target: &Path) -> io::Result<()> {
    for entry in files {
        materialize_file(entry, manifest, target).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "cannot materialize assembly file {}: {error}",
                    entry.path.display()
                ),
            )
        })?;
    }
    Ok(())
}

fn materialize_file(entry: &AssemblyFile, manifest: &Path, target: &Path) -> io::Result<()> {
    let path = prepare_path(target, &entry.path)?;
    match &entry.kind {
        AssemblyFileKind::Directory { .. } => create_directory(&path)?,
        kind => {
            replace_file(&path, entry.replace)?;
            match kind {
                AssemblyFileKind::Content { content } => write_new(&path, content.as_bytes())?,
                AssemblyFileKind::Symlink { symlink: link } => symlink(link, &path)?,
                AssemblyFileKind::Source { source } => {
                    copy_new(&nex_source::local_path(manifest, source), &path)?;
                }
                AssemblyFileKind::Empty {} => write_new(&path, b"")?,
                AssemblyFileKind::Directory { .. } => unreachable!(),
            }
        }
    }
    if let Some(mode) = entry
        .mode
        .filter(|_| !matches!(entry.kind, AssemblyFileKind::Symlink { .. }))
    {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    Ok(())
}

fn replace_file(path: &Path, replace_directory: bool) -> io::Result<()> {
    let metadata = match path.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() && !replace_directory {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "assembly file conflicts with directory {} (use replace: true)",
                path.display()
            ),
        ));
    }
    remove_entry(path)
}

fn create_directory(path: &Path) -> io::Result<()> {
    match path.symlink_metadata() {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("assembly directory conflicts with {}", path.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(path),
        Err(error) => Err(error),
    }
}

fn prepare_path(root: &Path, value: &Path) -> io::Result<PathBuf> {
    let relative = value.strip_prefix("/").expect("validated assembly path");
    let mut path = root.to_path_buf();
    let mut parts = relative.components().peekable();
    while let Some(Component::Normal(part)) = parts.next() {
        path.push(part);
        if parts.peek().is_some() {
            match path.symlink_metadata() {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("assembly path traverses symlink: {}", value.display()),
                    ));
                }
                Ok(metadata) if !metadata.is_dir() => {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        format!(
                            "assembly path parent is not a directory: {}",
                            path.display()
                        ),
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&path)?,
                Err(error) => return Err(error),
            }
        }
    }
    Ok(path)
}

fn copy_new(source: &Path, destination: &Path) -> io::Result<()> {
    let mut input = fs::File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    io::copy(&mut input, &mut output)?;
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)
}

fn remove_entry(path: &Path) -> io::Result<()> {
    let metadata = path.symlink_metadata()?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn publish(
    repo: &Repo,
    target: &Path,
    reference: &str,
    checksum: &str,
    recipe: &str,
) -> io::Result<()> {
    let sequence = ASSEMBLY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = format!("nex/tmp/assembly/{}/{sequence}", std::process::id());
    let metadata = metadata::build(checksum, recipe);
    let result = (|| {
        let hash = zub::ops::commit_with_metadata(
            repo,
            target,
            &temporary,
            Some(""),
            Some(metadata::AUTHOR),
            &metadata,
        )
        .map_err(io::Error::other)?;
        zub::write_ref(repo, reference, &hash).map_err(io::Error::other)
    })();
    let _ = zub::delete_ref(repo, &temporary);
    result
}

fn verify_checksum(manifest: &AssemblyManifest, actual: &str) -> Result<()> {
    match manifest.system.checksum.as_deref() {
        Some(expected) if expected != actual => Err(Error::ChecksumMismatch {
            kind: ChecksumKind::System,
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        }),
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "assembly_tests.rs"]
mod tests;
