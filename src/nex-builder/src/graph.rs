//! Complete, immutable build graphs loaded before workers start.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use zub::Repo;

use crate::assembly_manifest::{load_assembly, AssemblyManifest, PackagePlacement};
use crate::catalog;
use crate::manifest::{
    load_environment, load_manifest, BuildEnvironment, Dependency, PackageManifest,
};
use crate::reference::{InputRef, PackageKey, PackageRef};
use crate::runtime::{self, Providers};
use crate::schema::invalid;
use crate::{Error, Result};

pub(crate) struct BuildGraph {
    pub nodes: Vec<Node>,
    pub root: usize,
}

pub(crate) struct Node {
    pub label: String,
    pub manifest_path: PathBuf,
    pub environment: BuildEnvironment,
    pub dependencies: Vec<usize>,
    pub kind: NodeKind,
}

pub(crate) enum NodeKind {
    Package {
        manifest: PackageManifest,
        inputs: Vec<Dependency>,
    },
    Assembly {
        manifest: AssemblyManifest,
        build_inputs: Vec<Dependency>,
        packages: PackageLayout,
    },
}

pub(crate) enum PackageLayout {
    Flat(Vec<Dependency>),
    Nex(Vec<PackagePlan>),
}

pub(crate) struct PackagePlan {
    pub root: Dependency,
    pub runtime: Vec<Dependency>,
    pub placement: PackagePlacement,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum NodeKey {
    Package(PackageKey),
    Assembly(PathBuf),
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum VisitState {
    Visiting,
    Done(usize),
}

pub(crate) fn load_graph(manifest: &Path, repo: &Repo) -> Result<BuildGraph> {
    let catalog = catalog::root(manifest)?;
    let mut planner = Planner {
        catalog,
        repo,
        nodes: Vec::new(),
        states: BTreeMap::new(),
        stack: Vec::new(),
    };
    let root = match manifest_kind(manifest)? {
        ManifestKind::Package => planner.visit_package_path(manifest, None)?,
        ManifestKind::Assembly => planner.visit_assembly(manifest)?,
    };
    Ok(BuildGraph {
        nodes: planner.nodes,
        root,
    })
}

struct Planner<'a> {
    catalog: PathBuf,
    repo: &'a Repo,
    nodes: Vec<Node>,
    states: BTreeMap<NodeKey, VisitState>,
    stack: Vec<String>,
}

impl Planner<'_> {
    fn visit_package_ref(&mut self, reference: &PackageRef) -> Result<usize> {
        let key = NodeKey::Package(reference.key.clone());
        if self.states.contains_key(&key) {
            let index = self.existing_or_cycle(&key)?;
            let NodeKind::Package { manifest, .. } = &self.nodes[index].kind else {
                unreachable!("package key resolved to an assembly");
            };
            reference.validate_manifest(manifest)?;
            return Ok(index);
        }
        let path = catalog::package_manifest(&self.catalog, &reference.key);
        self.visit_package_path(&path, Some(reference))
    }

    fn visit_package_path(&mut self, path: &Path, requested: Option<&PackageRef>) -> Result<usize> {
        let path = path.canonicalize().map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("cannot load package manifest {}: {error}", path.display()),
            )
        })?;
        let manifest = load_manifest(&path)?;
        let key = PackageKey::from_manifest(&manifest);
        if let Some(reference) = requested {
            reference.validate_manifest(&manifest)?;
        }
        let node_key = NodeKey::Package(key.clone());
        if self.states.contains_key(&node_key) {
            return self.existing_or_cycle(&node_key);
        }
        self.states.insert(node_key.clone(), VisitState::Visiting);
        self.stack.push(format!("package {key}"));

        let environment = load_environment(&path, &manifest.build.environment)?;
        let mut dependencies = BTreeSet::new();
        for dependency in &manifest.dependencies {
            match &dependency.commit {
                InputRef::Package(reference) => {
                    dependencies.insert(self.visit_package_ref(reference)?);
                }
                InputRef::Stored(hash) => self.require_stored(hash)?,
            }
        }
        let inputs = runtime::resolve(&manifest.dependencies, Providers::Fallback, |reference| {
            self.package(reference)
        })?;
        self.stack.pop();
        let index = self.nodes.len();
        self.nodes.push(Node {
            label: format!("package {key}"),
            manifest_path: path,
            environment,
            dependencies: dependencies.into_iter().collect(),
            kind: NodeKind::Package { manifest, inputs },
        });
        self.states.insert(node_key, VisitState::Done(index));
        Ok(index)
    }

    fn visit_assembly(&mut self, path: &Path) -> Result<usize> {
        let path = path.canonicalize().map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("cannot load assembly manifest {}: {error}", path.display()),
            )
        })?;
        let key = NodeKey::Assembly(path.clone());
        if self.states.contains_key(&key) {
            return self.existing_or_cycle(&key);
        }
        let manifest = load_assembly(&path)?;
        let label = format!(
            "system {}/{}",
            manifest.system.slug, manifest.system.version
        );
        self.states.insert(key.clone(), VisitState::Visiting);
        self.stack.push(label.clone());

        let environment = load_environment(&path, &manifest.build.environment)?;
        let dependencies = self.visit_assembly_dependencies(&manifest)?;
        let build_inputs =
            runtime::resolve(&manifest.dependencies, Providers::Fallback, |reference| {
                self.package(reference)
            })?;
        let packages = self.plan_assembly_packages(&manifest)?;
        self.stack.pop();
        let index = self.nodes.len();
        self.nodes.push(Node {
            label,
            manifest_path: path,
            environment,
            dependencies: dependencies.into_iter().collect(),
            kind: NodeKind::Assembly {
                manifest,
                build_inputs,
                packages,
            },
        });
        self.states.insert(key, VisitState::Done(index));
        Ok(index)
    }

    fn plan_assembly_packages(&self, manifest: &AssemblyManifest) -> Result<PackageLayout> {
        let packages = manifest
            .packages
            .iter()
            .map(|package| package.dependency.clone())
            .collect::<Vec<_>>();
        let providers = Providers::Required(&manifest.providers, &packages);
        if !manifest.system.nex_structure {
            return Ok(
                runtime::resolve(&packages, providers, |reference| self.package(reference))
                    .map(PackageLayout::Flat)?,
            );
        }
        manifest
            .packages
            .iter()
            .map(|package| {
                let resolved =
                    runtime::resolve_root(&package.dependency, providers, |reference| {
                        self.package(reference)
                    })?;
                Ok(PackagePlan {
                    root: resolved.root,
                    runtime: resolved.runtime,
                    placement: package.placement.unwrap_or(PackagePlacement::Capsule),
                })
            })
            .collect::<Result<Vec<_>>>()
            .map(PackageLayout::Nex)
    }

    fn visit_assembly_dependencies(
        &mut self,
        manifest: &AssemblyManifest,
    ) -> Result<BTreeSet<usize>> {
        let mut dependencies = BTreeSet::new();
        if let Some(base) = &manifest.base {
            let base_path = self.resolve_manifest(&base.manifest);
            let index = self.visit_assembly(&base_path)?;
            let actual = self.assembly(index).reference();
            if base.commit != actual {
                return Err(invalid(format!(
                    "base manifest {} publishes {actual}, not {}",
                    base.manifest.display(),
                    base.commit
                ))
                .into());
            }
            dependencies.insert(index);
        }
        for dependency in manifest
            .dependencies
            .iter()
            .chain(manifest.packages.iter().map(|package| &package.dependency))
        {
            match &dependency.commit {
                InputRef::Package(reference) => {
                    dependencies.insert(self.visit_package_ref(reference)?);
                }
                InputRef::Stored(hash) => self.require_stored(hash)?,
            }
        }
        Ok(dependencies)
    }

    fn existing_or_cycle(&self, key: &NodeKey) -> Result<usize> {
        match self.states.get(key) {
            Some(VisitState::Done(index)) => Ok(*index),
            Some(VisitState::Visiting) => {
                let mut chain = self.stack.join(" -> ");
                if !chain.is_empty() {
                    chain.push_str(" -> ");
                }
                chain.push_str(&key.to_string());
                Err(Error::GraphCycle { chain })
            }
            None => unreachable!("node state checked before lookup"),
        }
    }

    fn require_stored(&self, reference: &str) -> Result<()> {
        let result = (|| {
            let hash = zub::resolve_ref(self.repo, reference)?;
            zub::read_commit(self.repo, &hash)?;
            Ok(())
        })();
        result.map_err(|source| Error::MissingInput {
            reference: reference.to_owned(),
            source,
        })
    }

    fn resolve_manifest(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.catalog.join(path)
        }
    }

    fn assembly(&self, index: usize) -> &AssemblyManifest {
        match &self.nodes[index].kind {
            NodeKind::Assembly { manifest, .. } => manifest,
            NodeKind::Package { .. } => unreachable!("base node is an assembly"),
        }
    }

    fn package(&self, reference: &PackageRef) -> io::Result<&PackageManifest> {
        let key = NodeKey::Package(reference.key.clone());
        let Some(VisitState::Done(index)) = self.states.get(&key) else {
            return Err(invalid(format!("package {} is not ready", reference.key)));
        };
        match &self.nodes[*index].kind {
            NodeKind::Package { manifest, .. } => Ok(manifest),
            NodeKind::Assembly { .. } => unreachable!("package key resolved to an assembly"),
        }
    }
}

impl std::fmt::Display for NodeKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Package(key) => write!(formatter, "package {key}"),
            Self::Assembly(path) => write!(formatter, "assembly {}", path.display()),
        }
    }
}

enum ManifestKind {
    Package,
    Assembly,
}

fn manifest_kind(path: &Path) -> io::Result<ManifestKind> {
    let value: serde_yaml::Value = serde_yaml::from_slice(&fs::read(path)?)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let mapping = value
        .as_mapping()
        .ok_or_else(|| invalid("manifest root must be a YAML mapping"))?;
    let package = mapping.contains_key(serde_yaml::Value::String("package".to_string()));
    let system = mapping.contains_key(serde_yaml::Value::String("system".to_string()));
    match (package, system) {
        (true, false) => Ok(ManifestKind::Package),
        (false, true) => Ok(ManifestKind::Assembly),
        _ => Err(invalid(
            "manifest must declare exactly one of package or system",
        )),
    }
}
