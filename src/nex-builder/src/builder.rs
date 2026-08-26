//! Top-level package build flow.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::num::NonZeroUsize;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use zub::ops::CheckoutOptions;
use zub::{EntryKind, Hash, Repo};

use crate::checksum::output_checksum;
use crate::manifest::{BuildEnvironment, Dependency, PackageManifest};
use crate::output::{files_ref, publish};
use crate::progress::BuildPass;
use crate::sandbox::{self, JobOutput};
use crate::source::stage_sources;
use crate::{ChecksumKind, Error, Result};

pub(crate) struct BuildContext<'a> {
    pub repo: &'a Repo,
    pub environment: &'a BuildEnvironment,
    pub manifest_path: &'a Path,
    pub source_cache: &'a Path,
    pub build_root: &'a Path,
    pub check: bool,
    pub cpu_count: NonZeroUsize,
    pub recipe: &'a str,
    pub output: Option<&'a JobOutput>,
}

pub(crate) fn run_package(
    manifest: &PackageManifest,
    inputs: &[Dependency],
    context: &BuildContext<'_>,
) -> Result<()> {
    let checksum = build_once(
        manifest,
        inputs,
        context,
        context.build_root,
        BuildPass::Primary,
    )?;
    verify_expected_checksum(manifest, &checksum)?;
    verify_reproducible(&checksum, context, |root, pass| {
        build_once(manifest, inputs, context, root, pass)
    })?;
    let output_root = context.build_root.join(&context.environment.paths.out);
    publish(
        context.repo,
        manifest,
        &output_root,
        &checksum,
        context.recipe,
    )
    .map_err(|source| Error::Publication {
        reference: files_ref(manifest),
        source,
    })
}

pub(crate) fn verify_reproducible(
    first: &str,
    context: &BuildContext<'_>,
    build: impl FnOnce(&Path, BuildPass) -> io::Result<String>,
) -> Result<()> {
    if !context.check {
        return Ok(());
    }
    let second = build(
        &second_build_root(context.build_root),
        BuildPass::Reproducibility,
    )?;
    if first == second {
        Ok(())
    } else {
        Err(Error::ChecksumMismatch {
            kind: ChecksumKind::Reproducibility,
            expected: first.to_owned(),
            actual: second,
        })
    }
}

fn build_once(
    manifest: &PackageManifest,
    inputs: &[Dependency],
    context: &BuildContext<'_>,
    root: &Path,
    pass: BuildPass,
) -> io::Result<String> {
    reset_root(root, context.environment)?;
    let root = root.canonicalize()?;
    checkout_dependencies(context.repo, inputs, &root)?;
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
    output_checksum(&root.join(&context.environment.paths.out))
}

pub(crate) fn reset_root(root: &Path, environment: &BuildEnvironment) -> io::Result<()> {
    if root.exists() {
        if !root.join(".nex-builder-root").is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "refusing to clear unowned build directory {}",
                    root.display()
                ),
            ));
        }
        fs::remove_dir_all(root)?;
    }
    fs::create_dir_all(root)?;
    fs::write(root.join(".nex-builder-root"), b"")?;
    fs::create_dir_all(root.join(&environment.paths.work))?;
    fs::create_dir_all(root.join(&environment.paths.out))?;
    fs::create_dir_all(root.join(&environment.paths.inputs))?;
    fs::create_dir_all(root.join("tmp"))?;
    Ok(())
}

pub(crate) fn checkout_dependencies(
    repo: &Repo,
    dependencies: &[crate::manifest::Dependency],
    root: &Path,
) -> io::Result<()> {
    for dependency in dependencies {
        let tree = dependency_tree(repo, dependency)?;
        zub::ops::checkout_from_tree_hash(repo, &tree, root, checkout_options()).map_err(
            |error| io::Error::other(format!("cannot checkout {}: {error}", dependency.commit)),
        )?;
    }
    Ok(())
}

pub(crate) fn checkout_reference(repo: &Repo, reference: &str, root: &Path) -> io::Result<()> {
    zub::ops::checkout(repo, reference, root, checkout_options())
        .map_err(|error| io::Error::other(format!("cannot checkout {reference}: {error}")))
}

pub(crate) fn dependency_tree(repo: &Repo, dependency: &Dependency) -> io::Result<Hash> {
    let reference = dependency.commit.to_string();
    let root = reference_tree(repo, &reference)?;
    if dependency.paths.is_empty() {
        return Ok(root);
    }
    let paths = include_symlink_targets(repo, &root, &dependency.paths)?;
    zub::ops::select_tree_from_hash(repo, &root, &paths)
        .map_err(|error| io::Error::other(error.to_string()))
}

pub(crate) fn reference_tree(repo: &Repo, reference: &str) -> io::Result<Hash> {
    let hash = zub::resolve_ref(repo, reference)
        .map_err(|error| io::Error::other(format!("cannot resolve {reference}: {error}")))?;
    zub::read_commit(repo, &hash)
        .map(|commit| commit.tree)
        .map_err(|error| io::Error::other(error.to_string()))
}

fn include_symlink_targets(
    repo: &Repo,
    root: &Hash,
    paths: &[PathBuf],
) -> io::Result<Vec<PathBuf>> {
    let mut selected = BTreeSet::new();
    for path in paths {
        let mut path = normalize(path)?;
        let mut chain = BTreeSet::new();
        loop {
            if !chain.insert(path.clone()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("symlink cycle at {}", path.display()),
                ));
            }
            selected.insert(path.clone());
            let EntryKind::Symlink { hash, .. } = entry_kind(repo, root, &path)? else {
                break;
            };
            let target =
                zub::read_blob(repo, &hash).map_err(|error| io::Error::other(error.to_string()))?;
            let target = PathBuf::from(OsString::from_vec(target));
            path = normalize(&path.parent().unwrap_or(Path::new("/")).join(target))?;
        }
    }
    Ok(selected.into_iter().collect())
}

fn entry_kind(repo: &Repo, root: &Hash, path: &Path) -> io::Result<EntryKind> {
    let mut tree =
        zub::read_tree(repo, root).map_err(|error| io::Error::other(error.to_string()))?;
    let mut parts = path.components().filter_map(|part| match part {
        std::path::Component::Normal(name) => name.to_str(),
        _ => None,
    });
    let mut part = parts.next().ok_or_else(|| invalid_path(path))?;
    loop {
        let entry = tree.get(part).ok_or_else(|| invalid_path(path))?;
        let Some(next) = parts.next() else {
            return Ok(entry.kind.clone());
        };
        let EntryKind::Directory { hash, .. } = &entry.kind else {
            return Err(invalid_path(path));
        };
        tree = zub::read_tree(repo, hash).map_err(|error| io::Error::other(error.to_string()))?;
        part = next;
    }
}

fn normalize(path: &Path) -> io::Result<PathBuf> {
    let mut clean = PathBuf::from("/");
    for part in path.components() {
        match part {
            std::path::Component::RootDir | std::path::Component::CurDir => {}
            std::path::Component::Normal(name) => clean.push(name),
            std::path::Component::ParentDir if clean.pop() => {}
            _ => return Err(invalid_path(path)),
        }
    }
    if clean.file_name().is_some() {
        Ok(clean)
    } else {
        Err(invalid_path(path))
    }
}

fn invalid_path(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid selected path {}", path.display()),
    )
}

fn checkout_options() -> CheckoutOptions {
    CheckoutOptions {
        force: true,
        hardlink: false,
        preserve_sparse: false,
    }
}

fn verify_expected_checksum(manifest: &PackageManifest, actual: &str) -> Result<()> {
    let Some(expected) = manifest.package.checksum.as_deref() else {
        return Ok(());
    };
    if expected == actual {
        Ok(())
    } else {
        Err(Error::ChecksumMismatch {
            kind: ChecksumKind::PackageOutput,
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        })
    }
}

pub(crate) fn open_repo(path: &Path) -> Result<Repo> {
    if path.exists() {
        Repo::open(path)
    } else {
        Repo::init(path)
    }
    .map_err(Error::from)
}

fn second_build_root(first: &Path) -> PathBuf {
    let name = first
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("build");
    first.with_file_name(format!("{name}-check"))
}

pub(crate) fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

pub(crate) fn host_cpu_count() -> NonZeroUsize {
    std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
