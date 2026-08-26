//! Nex system layout with isolated package capsules and public filesystem links.

use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;
use zub::Repo;

use crate::assembly_manifest::PackagePlacement;
use crate::builder::checkout_dependencies;
use crate::catalog;
use crate::graph::PackagePlan;
use crate::manifest::Dependency;
use crate::metadata;
use crate::nex_links::{create_fhs_links, link_capsule, link_libraries};
use crate::reference::{InputRef, PackageRef};
use crate::schema::{invalid, validate_hash};

const LOADER_NAME: &str = "ld-linux-x86-64.so.2";
const LOADER_SHIM: &str = "usr/lib/nex-ld-shim";
const SYSTEM_LOADER_DIRECTORY: &str = "lib64";

struct Capsule<'a> {
    plan: &'a PackagePlan,
    path: PathBuf,
    logical: PathBuf,
}

pub(crate) fn materialize(
    repo: &Repo,
    plans: &[PackagePlan],
    catalog: &Path,
    build_root: &Path,
    target: &Path,
) -> io::Result<()> {
    let package_root = target.join("nex/pkg");
    fs::create_dir_all(&package_root)?;
    deploy_catalog(catalog, target)?;
    let capsules = install_roots(repo, plans, &package_root, target)?;
    flatten_runtime(repo, &capsules, build_root)?;
    link_libraries(&package_root, target)?;
    install_loader_shim(build_root, target)?;
    create_fhs_links(target)
}

fn install_roots<'a>(
    repo: &Repo,
    plans: &'a [PackagePlan],
    package_root: &Path,
    target: &Path,
) -> io::Result<Vec<Capsule<'a>>> {
    let mut capsules = Vec::new();
    for plan in plans {
        if plan.placement == PackagePlacement::Root {
            checkout_dependencies(repo, std::slice::from_ref(&plan.root), target)?;
            continue;
        }
        let reference = package_reference(&plan.root)?;
        let relative = capsule_relative(repo, reference)?;
        let capsule = Capsule {
            plan,
            path: package_root.join(&relative),
            logical: Path::new("/nex/pkg").join(relative),
        };
        fs::create_dir_all(&capsule.path)?;
        checkout_dependencies(repo, std::slice::from_ref(&plan.root), &capsule.path)?;
        record_root(&capsule.path, &plan.root)?;
        link_capsule(&capsule.path, &capsule.logical, target)?;
        capsules.push(capsule);
    }
    Ok(capsules)
}

fn flatten_runtime(repo: &Repo, capsules: &[Capsule<'_>], build_root: &Path) -> io::Result<()> {
    let scratch_root = build_root.join("tmp");
    fs::create_dir_all(&scratch_root)?;
    for capsule in capsules {
        if capsule.plan.runtime.is_empty() {
            continue;
        }
        let scratch = tempfile::tempdir_in(&scratch_root)?;
        checkout_dependencies(repo, &capsule.plan.runtime, scratch.path())?;
        merge_missing(scratch.path(), &capsule.path)?;
        ensure_capsule_loader(&capsule.path)?;
    }
    Ok(())
}

fn package_reference(dependency: &Dependency) -> io::Result<&PackageRef> {
    match &dependency.commit {
        InputRef::Package(reference) => Ok(reference),
        InputRef::Stored(_) => Err(invalid(format!(
            "Nex layout requires a package reference, got {}",
            dependency.commit
        ))),
    }
}

fn capsule_relative(repo: &Repo, reference: &PackageRef) -> io::Result<PathBuf> {
    let value = reference.to_string();
    let hash = zub::resolve_ref(repo, &value)
        .map_err(|error| io::Error::other(format!("cannot resolve {reference}: {error}")))?;
    let commit = zub::read_commit(repo, &hash).map_err(io::Error::other)?;
    let checksum = commit.metadata.get(metadata::CHECKSUM).ok_or_else(|| {
        invalid(format!(
            "{reference} has no {} metadata",
            metadata::CHECKSUM
        ))
    })?;
    validate_hash(checksum, "stored package checksum")?;
    let short = checksum
        .get(..8)
        .ok_or_else(|| invalid("stored package checksum is too short"))?;
    Ok(Path::new(&reference.key.namespace)
        .join(&reference.key.slug)
        .join(&reference.key.version)
        .join(short))
}

fn record_root(capsule: &Path, root: &Dependency) -> io::Result<()> {
    let marker = capsule.join(".nex-app-root");
    let mut roots = match fs::read_to_string(&marker) {
        Ok(value) => value.lines().map(str::to_owned).collect::<Vec<_>>(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error),
    };
    let root = root.commit.to_string();
    if !roots.contains(&root) {
        roots.push(root);
        fs::write(marker, format!("{}\n", roots.join("\n")))?;
    }
    Ok(())
}

fn merge_missing(source: &Path, destination: &Path) -> io::Result<()> {
    for entry in WalkDir::new(source).min_depth(1).sort_by_file_name() {
        let entry = entry?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .expect("walk stays under source");
        let target = destination.join(relative);
        if target.symlink_metadata().is_ok() {
            continue;
        }
        if entry.file_type().is_dir() {
            fs::create_dir(&target)?;
            fs::set_permissions(&target, entry.metadata()?.permissions())?;
        } else if entry.file_type().is_symlink() {
            symlink(fs::read_link(entry.path())?, target)?;
        } else if entry.file_type().is_file() {
            if fs::hard_link(entry.path(), &target).is_err() {
                fs::copy(entry.path(), target)?;
            }
        } else {
            return Err(invalid(format!(
                "unsupported runtime file {}",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

fn ensure_capsule_loader(capsule: &Path) -> io::Result<()> {
    let loader = capsule.join("usr/lib").join(LOADER_NAME);
    let link = capsule.join("lib").join(LOADER_NAME);
    if loader.symlink_metadata().is_ok() && link.symlink_metadata().is_err() {
        fs::create_dir_all(link.parent().expect("loader link has parent"))?;
        symlink(Path::new("../usr/lib").join(LOADER_NAME), link)?;
    }
    Ok(())
}

fn deploy_catalog(catalog: &Path, target: &Path) -> io::Result<()> {
    let destination = target.join("nex/db/pkg");
    if destination.exists() {
        fs::remove_dir_all(&destination)?;
    }
    fs::create_dir_all(&destination)?;
    catalog::visit_manifests(catalog, |path, relative| {
        let output = destination.join(relative);
        fs::create_dir_all(output.parent().expect("manifest has parent"))?;
        fs::copy(path, output)?;
        Ok(())
    })
}

fn install_loader_shim(build_root: &Path, target: &Path) -> io::Result<()> {
    let source = build_root.join(LOADER_SHIM);
    let directory = target.join(SYSTEM_LOADER_DIRECTORY);
    let destination = directory.join(LOADER_NAME);
    fs::create_dir_all(&directory)?;
    if destination.exists() {
        if !source.exists() || fs::read(&source)? == fs::read(&destination)? {
            return Ok(());
        }
        return Err(invalid(format!(
            "base system has a different loader shim at {}",
            destination.display()
        )));
    }
    fs::copy(&source, &destination)
        .map(|_| ())
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "cannot install Nex loader shim {}: {error}",
                    source.display()
                ),
            )
        })
}

#[cfg(test)]
#[path = "nex_layout_tests.rs"]
mod tests;
