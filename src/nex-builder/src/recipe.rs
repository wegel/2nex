//! Build-recipe identities for safe reuse without stable output checksums.

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serde::Serialize;
use zub::Repo;

use crate::builder::{dependency_tree, reference_tree};
use crate::catalog;
use crate::graph::{Node, NodeKind, PackageLayout};
use crate::manifest::Dependency;

const VERSION: &[u8] = b"nex-builder.recipe.1\0";

pub(crate) fn compute(repo: &Repo, node: &Node) -> io::Result<String> {
    let mut hash = blake3::Hasher::new();
    hash.update(VERSION);
    match &node.kind {
        NodeKind::Package { manifest, inputs } => {
            write(&mut hash, manifest)?;
            add_dependencies(&mut hash, repo, inputs)?;
        }
        NodeKind::Assembly {
            manifest,
            build_inputs,
            packages,
        } => {
            write(&mut hash, manifest)?;
            if manifest.system.nex_structure {
                add_catalog(&mut hash, &catalog::root(&node.manifest_path)?)?;
            }
            if let Some(base) = &manifest.base {
                add_reference(&mut hash, repo, &base.commit)?;
            }
            add_dependencies(&mut hash, repo, build_inputs)?;
            add_packages(&mut hash, repo, packages)?;
        }
    }
    write(&mut hash, &node.environment)?;
    Ok(hash.finalize().to_hex().to_string())
}

fn add_catalog(hash: &mut blake3::Hasher, catalog: &Path) -> io::Result<()> {
    catalog::visit_manifests(catalog, |path, relative| {
        hash.update(relative.as_os_str().as_encoded_bytes());
        hash.update(b"\0");
        let mode = path.metadata()?.permissions().mode() & 0o7777;
        hash.update(&mode.to_le_bytes());
        hash.update(&fs::read(path)?);
        hash.update(b"\0");
        Ok(())
    })
}

fn add_packages(hash: &mut blake3::Hasher, repo: &Repo, layout: &PackageLayout) -> io::Result<()> {
    match layout {
        PackageLayout::Flat(inputs) => add_dependencies(hash, repo, inputs),
        PackageLayout::Nex(capsules) => {
            for capsule in capsules {
                add_dependencies(hash, repo, &capsule.runtime)?;
                add_dependencies(hash, repo, std::slice::from_ref(&capsule.root))?;
            }
            Ok(())
        }
    }
}

fn write(hash: &mut blake3::Hasher, value: &impl Serialize) -> io::Result<()> {
    serde_yaml::to_writer(&mut *hash, value).map_err(io::Error::other)?;
    hash.update(b"\0");
    Ok(())
}

fn add_dependencies(
    hash: &mut blake3::Hasher,
    repo: &Repo,
    dependencies: &[Dependency],
) -> io::Result<()> {
    for dependency in dependencies {
        let tree = dependency_tree(repo, dependency)?;
        hash.update(tree.to_string().as_bytes());
    }
    Ok(())
}

fn add_reference(hash: &mut blake3::Hasher, repo: &Repo, reference: &str) -> io::Result<()> {
    let tree = reference_tree(repo, reference)?;
    hash.update(tree.to_string().as_bytes());
    Ok(())
}

#[cfg(test)]
#[path = "recipe_tests.rs"]
mod tests;
