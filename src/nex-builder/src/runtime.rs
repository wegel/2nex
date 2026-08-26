//! Runtime inputs selected by package `needs` and `resolution` data.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::manifest::{Dependency, PackageManifest, ResolutionTarget};
use crate::reference::{InputRef, PackageRef, RefKind};
use crate::schema::{invalid, validate_name};

#[derive(Clone, Copy)]
pub(crate) enum Providers<'a> {
    Fallback,
    Required(&'a BTreeMap<String, InputRef>, &'a [Dependency]),
}

pub(crate) struct ResolvedRoot {
    pub root: Dependency,
    pub runtime: Vec<Dependency>,
}

pub(crate) fn validate(manifest: &PackageManifest) -> io::Result<()> {
    for (path, target) in &manifest.resolution {
        validate_absolute(path, "resolved runtime path")?;
        match target {
            ResolutionTarget::Dependency(name) if name == "self" => {}
            ResolutionTarget::Dependency(name) => {
                require_dependency(manifest, name)?;
            }
            ResolutionTarget::Capability {
                capability,
                fallback,
            } => {
                validate_capability(capability)?;
                if let Some(name) = fallback {
                    require_dependency(manifest, name)?;
                }
            }
        }
    }
    for entry in manifest.outputs.values().flat_map(|output| &output.files) {
        let mut seen = BTreeSet::new();
        for need in &entry.needs {
            validate_absolute(need, "runtime need")?;
            if !seen.insert(need) {
                return Err(invalid(format!(
                    "{} lists runtime need {need} more than once",
                    entry.path
                )));
            }
            if !manifest.resolution.contains_key(need) {
                return Err(invalid(format!(
                    "{} needs {need}, but resolution does not name its provider",
                    entry.path
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_providers(
    packages: &[Dependency],
    providers: &BTreeMap<String, InputRef>,
) -> io::Result<()> {
    for (capability, commit) in providers {
        validate_capability(capability)?;
        if !packages.iter().any(|package| package.commit == *commit) {
            return Err(invalid(format!(
                "provider {capability} refers to {commit}, which is not an assembly package"
            )));
        }
    }
    Ok(())
}

pub(crate) fn resolve<'a>(
    direct: &[Dependency],
    providers: Providers<'a>,
    manifest: impl Fn(&PackageRef) -> io::Result<&'a PackageManifest>,
) -> io::Result<Vec<Dependency>> {
    let seen = direct
        .iter()
        .map(|dependency| (dependency.commit.clone(), dependency.paths.clone()))
        .collect();
    let mut resolver = Resolver {
        manifest,
        providers,
        seen,
        inputs: Vec::new(),
    };
    for dependency in direct {
        resolver.visit_children(dependency)?;
    }
    resolver.inputs.extend_from_slice(direct);
    Ok(resolver.inputs)
}

pub(crate) fn resolve_root<'a>(
    root: &Dependency,
    providers: Providers<'a>,
    manifest: impl Fn(&PackageRef) -> io::Result<&'a PackageManifest>,
) -> io::Result<ResolvedRoot> {
    let mut resolver = Resolver {
        manifest,
        providers,
        seen: [(root.commit.clone(), root.paths.clone())].into(),
        inputs: Vec::new(),
    };
    resolver.visit_children(root)?;
    Ok(ResolvedRoot {
        root: root.clone(),
        runtime: resolver.inputs,
    })
}

struct Resolver<'a, F> {
    manifest: F,
    providers: Providers<'a>,
    seen: BTreeSet<(InputRef, Vec<PathBuf>)>,
    inputs: Vec<Dependency>,
}

impl<'a, F> Resolver<'a, F>
where
    F: Fn(&PackageRef) -> io::Result<&'a PackageManifest>,
{
    fn visit(&mut self, dependency: &Dependency) -> io::Result<()> {
        let key = (dependency.commit.clone(), dependency.paths.clone());
        if self.seen.iter().any(|seen| covers(seen, &key)) {
            return Ok(());
        }
        self.seen.insert(key);
        self.visit_children(dependency)?;
        self.inputs.push(dependency.clone());
        Ok(())
    }

    fn visit_children(&mut self, dependency: &Dependency) -> io::Result<()> {
        if let InputRef::Package(reference) = &dependency.commit {
            let dependencies = {
                let manifest = (self.manifest)(reference)?;
                runtime_dependencies(manifest, reference, &dependency.paths, self.providers)?
            };
            for dependency in &dependencies {
                self.visit(dependency)?;
            }
        }
        Ok(())
    }
}

fn covers(existing: &(InputRef, Vec<PathBuf>), candidate: &(InputRef, Vec<PathBuf>)) -> bool {
    existing.0 == candidate.0
        && (existing.1.is_empty()
            || (!candidate.1.is_empty()
                && candidate
                    .1
                    .iter()
                    .all(|path| existing.1.iter().any(|root| path.starts_with(root)))))
}

fn runtime_dependencies(
    manifest: &PackageManifest,
    reference: &PackageRef,
    paths: &[PathBuf],
    providers: Providers<'_>,
) -> io::Result<Vec<Dependency>> {
    let outputs = match &reference.kind {
        RefKind::Output(name) => vec![name.as_str()],
        RefKind::Bundle(name) => manifest
            .bundles
            .get(name)
            .expect("reference was validated")
            .iter()
            .map(String::as_str)
            .collect(),
        RefKind::Files => return Ok(Vec::new()),
    };
    let mut queue = VecDeque::new();
    for output in outputs {
        for entry in &manifest.outputs[output].files {
            if selected(&entry.path, paths) {
                queue.extend(entry.needs.iter().cloned());
            }
        }
    }
    resolve_needs(manifest, reference, providers, &mut queue)
}

fn resolve_needs(
    manifest: &PackageManifest,
    reference: &PackageRef,
    providers: Providers<'_>,
    queue: &mut VecDeque<String>,
) -> io::Result<Vec<Dependency>> {
    let mut seen = BTreeSet::new();
    let mut dependencies = Vec::new();
    while let Some(path) = queue.pop_front() {
        if !seen.insert(path.clone()) {
            continue;
        }
        match &manifest.resolution[&path] {
            ResolutionTarget::Dependency(name) if name == "self" => {
                let entry = manifest
                    .outputs
                    .values()
                    .flat_map(|output| &output.files)
                    .find(|entry| entry.path == path)
                    .ok_or_else(|| invalid(format!("self runtime file {path} is not an output")))?;
                queue.extend(entry.needs.iter().cloned());
                dependencies.push(Dependency {
                    name: Some("self".to_string()),
                    commit: InputRef::Package(PackageRef::new(
                        reference.key.clone(),
                        RefKind::Files,
                    )),
                    paths: vec![PathBuf::from(path)],
                });
            }
            ResolutionTarget::Dependency(name) => {
                dependencies.push(select_path(require_dependency(manifest, name)?, &path)?);
            }
            ResolutionTarget::Capability {
                capability,
                fallback,
            } => dependencies.push(select_path(
                &resolve_capability(manifest, capability, fallback.as_deref(), providers)?,
                &path,
            )?),
        }
    }
    Ok(dependencies)
}

fn select_path(dependency: &Dependency, path: &str) -> io::Result<Dependency> {
    let path = PathBuf::from(path);
    let mut selected = dependency.clone();
    if selected.paths.is_empty() {
        selected.paths.push(path);
    } else if !selected
        .paths
        .iter()
        .any(|selection| path.starts_with(selection))
    {
        return Err(invalid(format!(
            "runtime path {} is outside dependency selection {}",
            path.display(),
            dependency.commit
        )));
    }
    Ok(selected)
}

fn resolve_capability(
    manifest: &PackageManifest,
    capability: &str,
    fallback: Option<&str>,
    providers: Providers<'_>,
) -> io::Result<Dependency> {
    match providers {
        Providers::Required(providers, packages) => {
            let commit = providers.get(capability).ok_or_else(|| {
                invalid(format!("assembly does not bind capability {capability}"))
            })?;
            Ok(packages
                .iter()
                .find(|package| package.commit == *commit)
                .expect("assembly providers were validated")
                .clone())
        }
        Providers::Fallback => {
            let name = fallback.ok_or_else(|| {
                invalid(format!("capability {capability} has no package fallback"))
            })?;
            Ok(require_dependency(manifest, name)?.clone())
        }
    }
}

fn require_dependency<'a>(manifest: &'a PackageManifest, name: &str) -> io::Result<&'a Dependency> {
    manifest
        .dependencies
        .iter()
        .find(|dependency| dependency.name.as_deref() == Some(name))
        .ok_or_else(|| invalid(format!("resolution names undeclared dependency {name}")))
}

fn selected(path: &str, selections: &[PathBuf]) -> bool {
    let path = Path::new(path);
    selections.is_empty()
        || selections
            .iter()
            .any(|selection| path.starts_with(selection) || selection.starts_with(path))
}

fn validate_absolute(path: &str, kind: &str) -> io::Result<()> {
    let path = Path::new(path);
    let escapes = path
        .components()
        .any(|part| matches!(part, Component::ParentDir | Component::Prefix(_)));
    if path.is_absolute() && path.file_name().is_some() && !escapes {
        Ok(())
    } else {
        Err(invalid(format!("invalid {kind} {path:?}")))
    }
}

fn validate_capability(capability: &str) -> io::Result<()> {
    for part in capability.split('.') {
        validate_name(part, "capability")?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
