//! Dependency graph construction for package and system builds.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::build::{check_if_built, compute_manifest_hash};
use crate::manifest::types::{Dependency, ManifestSource};
use crate::manifest::{
    compute_manifest_hash_from_source, load_manifest_from_source, repository_root_for_path,
    ManifestData,
};
use crate::refs::PackageRef;
use crate::store::Store;
use crate::system;
use crate::utils::short_hash;
use petgraph::graph::{DiGraph, NodeIndex};

use super::manifest_lookup::{build_exists_for_manifest, find_manifest_for_commit};

/// Inputs used to collect buildable dependency graph nodes.
pub struct DependencyGraphRequest<'a> {
    /// Root manifest to collect from.
    pub manifest_source: &'a ManifestSource,
    /// Primary zub repository path.
    pub repo_path: &'a str,
    /// Package manifest directories to search for floating dependencies.
    pub manifest_dirs: &'a [PathBuf],
    /// Graph to receive discovered manifest nodes.
    pub graph: &'a mut DiGraph<ManifestSource, ()>,
    /// Map from manifest paths to graph nodes.
    pub manifest_map: &'a mut HashMap<PathBuf, NodeIndex>,
    /// Rebuild packages even when the store has matching outputs.
    pub force: bool,
    /// Cache for store ref lookups.
    pub ref_cache: &'a mut HashMap<String, bool>,
    /// Open zub store, if one could be opened.
    pub store: Option<&'a Store>,
    /// Print verbose dependency lookup output.
    pub verbose: bool,
}

/// Recursively collect dependencies and add buildable nodes to the dependency graph.
pub fn collect_dependencies_recursive(
    request: DependencyGraphRequest<'_>,
) -> io::Result<NodeIndex> {
    GraphBuilder {
        repo_path: request.repo_path,
        manifest_dirs: request.manifest_dirs,
        graph: request.graph,
        manifest_map: request.manifest_map,
        force: request.force,
        ref_cache: request.ref_cache,
        store: request.store,
        verbose: request.verbose,
    }
    .collect(request.manifest_source)
}

struct GraphBuilder<'a> {
    repo_path: &'a str,
    manifest_dirs: &'a [PathBuf],
    graph: &'a mut DiGraph<ManifestSource, ()>,
    manifest_map: &'a mut HashMap<PathBuf, NodeIndex>,
    force: bool,
    ref_cache: &'a mut HashMap<String, bool>,
    store: Option<&'a Store>,
    verbose: bool,
}

struct ManifestSummary {
    slug: String,
    dependencies: Vec<Dependency>,
}

impl GraphBuilder<'_> {
    fn collect(&mut self, manifest_source: &ManifestSource) -> io::Result<NodeIndex> {
        let manifest_path = manifest_source.path().to_path_buf();
        if let Some(&node) = self.manifest_map.get(&manifest_path) {
            return Ok(node);
        }

        let manifest_data = load_manifest_from_source(manifest_source)?;
        let summary = summarize_manifest(&manifest_data);
        let node = self.add_manifest_node(manifest_source, &manifest_path);
        println!("Processing dependencies for {}", summary.slug);

        let deps_need_build = self.process_dependencies(&summary.dependencies, node)?;
        self.maybe_mark_package_skip(node, &manifest_data, &manifest_path, deps_need_build)?;

        Ok(node)
    }

    fn add_manifest_node(
        &mut self,
        manifest_source: &ManifestSource,
        manifest_path: &Path,
    ) -> NodeIndex {
        let node = self.graph.add_node(manifest_source.clone());
        self.manifest_map.insert(manifest_path.to_path_buf(), node);
        node
    }

    fn process_dependencies(
        &mut self,
        dependencies: &[Dependency],
        node: NodeIndex,
    ) -> io::Result<bool> {
        let mut deps_need_build = false;
        for dep in dependencies {
            if self.dependency_is_cached(dep)? {
                continue;
            }
            if self.add_dependency_edge(dep, node)? {
                deps_need_build = true;
            }
        }
        Ok(deps_need_build)
    }

    fn dependency_is_cached(&mut self, dep: &Dependency) -> io::Result<bool> {
        if let Some(revision) = dep.manifest_ref.as_ref() {
            self.pinned_dependency_is_cached(dep, revision)
        } else {
            self.floating_dependency_is_cached(dep)
        }
    }

    fn pinned_dependency_is_cached(
        &mut self,
        dep: &Dependency,
        revision: &str,
    ) -> io::Result<bool> {
        if !self.ref_is_available(&dep.commit) {
            self.print_needs_build(
                dep,
                &format!("pinned to {}, not in store", short_hash(revision)),
            );
            return Ok(false);
        }

        let manifest_path = match find_manifest_for_commit(&dep.commit, self.manifest_dirs) {
            Ok(path) => path,
            Err(error) => {
                self.print_pinned_revision_warning(dep, revision, &error);
                return Ok(false);
            }
        };
        let git_root = match repository_root_for_path(&manifest_path) {
            Ok(root) => root,
            Err(error) => {
                self.print_pinned_revision_warning(dep, revision, &error);
                return Ok(false);
            }
        };
        let manifest_source = ManifestSource::Repository {
            revision: revision.to_string(),
            path: manifest_path,
            git_root,
        };
        let content_hash = match compute_manifest_hash_from_source(&manifest_source) {
            Ok(hash) => hash,
            Err(e) => {
                eprintln!(
                    "  Warning: failed to fetch repository revision {} for {}: {}",
                    revision, dep.commit, e
                );
                return Ok(false);
            }
        };
        self.cached_for_hash(dep, &dep.commit, &content_hash, "pinned")
    }

    fn floating_dependency_is_cached(&mut self, dep: &Dependency) -> io::Result<bool> {
        let ref_to_check = commit_ref_for_availability(&dep.commit);
        if !self.ref_is_available(&ref_to_check) {
            return Ok(false);
        }

        let dep_manifest_path = match find_manifest_for_commit(&dep.commit, self.manifest_dirs) {
            Ok(path) => path,
            Err(_) => {
                self.print_floating_skip(dep, "no manifest found");
                return Ok(true);
            }
        };
        let manifest_hash = compute_manifest_hash(&dep_manifest_path)?;
        self.cached_for_hash(dep, &ref_to_check, &manifest_hash, "floating")
    }

    fn cached_for_hash(
        &self,
        dep: &Dependency,
        ref_to_check: &str,
        manifest_hash: &str,
        mode: &str,
    ) -> io::Result<bool> {
        let Some(store) = self.store else {
            return Ok(false);
        };
        match build_exists_for_manifest(store, ref_to_check, manifest_hash) {
            Ok(true) => {
                self.print_cached(dep, mode);
                Ok(true)
            }
            Ok(false) => {
                self.print_stale_or_needed(dep, mode);
                Ok(false)
            }
            Err(e) => {
                eprintln!(
                    "  Warning: failed to check manifest hash for {}: {}",
                    dep.commit, e
                );
                Ok(false)
            }
        }
    }

    fn add_dependency_edge(&mut self, dep: &Dependency, node: NodeIndex) -> io::Result<bool> {
        match find_manifest_for_commit(&dep.commit, self.manifest_dirs) {
            Ok(dep_manifest_path) => {
                self.add_manifest_dependency_edge(dep, dep_manifest_path, node)
            }
            Err(e) => Err(io::Error::new(
                e.kind(),
                format!(
                    "cannot build dependency {} because no manifest was found: {}",
                    dep.commit, e
                ),
            )),
        }
    }

    fn add_manifest_dependency_edge(
        &mut self,
        dep: &Dependency,
        dep_manifest_path: PathBuf,
        node: NodeIndex,
    ) -> io::Result<bool> {
        if let Some(dep_node) = self.manifest_map.get(&dep_manifest_path).copied() {
            self.add_edge_if_buildable(dep_node, node);
            return Ok(false);
        }

        if self.verbose {
            println!(
                "  Found dependency manifest: {}",
                dep_manifest_path.display()
            );
        }

        let dep_source = dependency_source(dep, dep_manifest_path)?;
        let dep_node = self.collect(&dep_source)?;
        Ok(self.add_edge_if_buildable(dep_node, node))
    }

    fn add_edge_if_buildable(&mut self, dep_node: NodeIndex, node: NodeIndex) -> bool {
        if self.graph[dep_node].is_skip() {
            return false;
        }
        self.graph.add_edge(dep_node, node, ());
        true
    }

    fn maybe_mark_package_skip(
        &mut self,
        node: NodeIndex,
        manifest_data: &ManifestData,
        manifest_path: &Path,
        deps_need_build: bool,
    ) -> io::Result<()> {
        if self.force || deps_need_build {
            return Ok(());
        }
        if let ManifestData::Package(manifest) = manifest_data {
            if check_if_built(self.repo_path, manifest, manifest_path)?.is_some() {
                println!("Package {} already built, skipping", manifest.package.slug);
                self.graph[node] = ManifestSource::Skip;
            }
        }
        Ok(())
    }

    fn ref_is_available(&mut self, ref_to_check: &str) -> bool {
        if let Some(available) = self.ref_cache.get(ref_to_check) {
            return *available;
        }
        let available = self
            .store
            .is_some_and(|store| store.resolve_ref(ref_to_check).is_ok());
        self.ref_cache.insert(ref_to_check.to_string(), available);
        available
    }

    fn print_cached(&self, dep: &Dependency, mode: &str) {
        if self.verbose {
            println!("  [{}] {} skipping", mode, dep.commit);
        }
    }

    fn print_stale_or_needed(&self, dep: &Dependency, mode: &str) {
        if self.verbose {
            println!(
                "  [{}/stale] {} manifest changed, rebuilding",
                mode, dep.commit
            );
        }
    }

    fn print_needs_build(&self, dep: &Dependency, reason: &str) {
        if self.verbose {
            println!("  [needs build] {} ({})", dep.commit, reason);
        }
    }

    fn print_floating_skip(&self, dep: &Dependency, reason: &str) {
        if self.verbose {
            println!("  [floating] {} skipping ({})", dep.commit, reason);
        }
    }

    fn print_pinned_revision_warning(&self, dep: &Dependency, revision: &str, error: &io::Error) {
        eprintln!(
            "  Warning: failed to locate repository revision {} for {}: {}",
            revision, dep.commit, error
        );
    }
}

fn summarize_manifest(manifest_data: &ManifestData) -> ManifestSummary {
    match manifest_data {
        ManifestData::Package(manifest) => ManifestSummary {
            slug: manifest.package.slug.clone(),
            dependencies: manifest.dependencies.clone(),
        },
        ManifestData::System(manifest) => {
            let mut dependencies = manifest.dependencies.clone();
            dependencies.extend(system::dependencies_from_system_packages(
                &manifest.packages,
            ));
            ManifestSummary {
                slug: manifest.system.slug.clone(),
                dependencies,
            }
        }
    }
}

fn commit_ref_for_availability(commit: &str) -> String {
    match PackageRef::parse(commit) {
        Ok(pkg_ref) if pkg_ref.has_internal_path() => pkg_ref.commit_ref(),
        _ => commit.to_string(),
    }
}

fn dependency_source(dep: &Dependency, path: PathBuf) -> io::Result<ManifestSource> {
    if let Some(revision) = dep.manifest_ref.as_ref() {
        let git_root = repository_root_for_path(&path)?;
        Ok(ManifestSource::Repository {
            revision: revision.clone(),
            path,
            git_root,
        })
    } else {
        Ok(ManifestSource::Path(path))
    }
}
