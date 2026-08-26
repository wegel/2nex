//! Bounded graph scheduling for packages and system assemblies.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use crate::builder::{absolute_path, host_cpu_count, open_repo};
use crate::graph::{load_graph, BuildGraph};
use crate::node_runner::{execute_node, node_references, Executor, NodeOutcome};
use crate::progress::{OutputOptions, Reporter};
use crate::{Error, Result};

// --- Public interface ---

/// Inputs, resource limits, and checks for one complete build graph.
#[derive(Clone, Debug)]
pub struct GraphOptions {
    /// Package or assembly manifest at the graph root.
    pub manifest: PathBuf,
    /// Zub repository used for dependencies and results.
    pub repo: PathBuf,
    /// Builder-owned roots and per-node logs.
    pub build_dir: PathBuf,
    /// Directory that holds source files by SHA-256 hash.
    pub source_cache: PathBuf,
    /// Whether to build each stale node twice and compare its trees.
    pub check: bool,
    /// Whether to attach current recipe metadata to complete older refs.
    pub adopt_existing: bool,
    /// Most build scripts that may run at once.
    pub jobs: NonZeroUsize,
    /// Total logical CPUs shared by concurrent build scripts.
    pub cpus: NonZeroUsize,
    /// Terminal output and optional timing-profile recording.
    pub output: OutputOptions,
}

/// Counts and root refs from a successful complete graph build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphResult {
    /// Nodes whose expected result was absent or stale.
    pub built: usize,
    /// Nodes reused after their refs, recipes, and checksums matched.
    pub reused: usize,
    /// Refs published by the root package or assembly.
    pub root_references: Vec<String>,
}

/// Build every stale node needed by one package or assembly manifest.
///
/// # Errors
///
/// Returns [`Error`] when a manifest, graph node, store operation, or worker
/// fails.
pub fn build_graph(options: &GraphOptions) -> Result<GraphResult> {
    let manifest = options.manifest.canonicalize()?;
    let repo_path = absolute_path(&options.repo)?;
    let repo = open_repo(&repo_path)?;
    let graph = load_graph(&manifest, &repo)?;
    let cpus = options.cpus;
    let workers = options
        .jobs
        .get()
        .min(cpus.get())
        .min(graph.nodes.len())
        .max(1);
    let reporter = Arc::new(Reporter::new(
        options.output,
        graph.nodes.len(),
        workers,
        cpus.get(),
    ));
    let executor = Executor {
        repo: repo_path,
        build_dir: absolute_path(&options.build_dir)?,
        source_cache: absolute_path(&options.source_cache)?,
        check: options.check,
        adopt_existing: options.adopt_existing,
        cpu_count: cpus,
        reporter: Arc::clone(&reporter),
    };
    let result = execute(&graph, &executor, workers);
    reporter.finish_graph(result.is_ok());
    result
}

impl GraphOptions {
    /// Resource defaults that keep the same graph path but run one node at a time.
    pub fn serial(
        manifest: PathBuf,
        repo: PathBuf,
        build_dir: PathBuf,
        source_cache: PathBuf,
        check: bool,
    ) -> Self {
        Self {
            manifest,
            repo,
            build_dir,
            source_cache,
            check,
            adopt_existing: false,
            jobs: NonZeroUsize::MIN,
            cpus: host_cpu_count(),
            output: OutputOptions::default(),
        }
    }
}

// --- Scheduler state ---

struct Scheduler {
    remaining: Vec<usize>,
    dependents: Vec<Vec<usize>>,
    ready: BTreeSet<usize>,
    built: usize,
    reused: usize,
    active: usize,
    completed: usize,
    used_cpus: usize,
    errors: BTreeMap<usize, Error>,
}

impl Scheduler {
    fn new(graph: &BuildGraph) -> Self {
        let remaining = graph
            .nodes
            .iter()
            .map(|node| node.dependencies.len())
            .collect::<Vec<_>>();
        let mut dependents = vec![Vec::new(); graph.nodes.len()];
        for (index, node) in graph.nodes.iter().enumerate() {
            for dependency in &node.dependencies {
                dependents[*dependency].push(index);
            }
        }
        let ready = remaining
            .iter()
            .enumerate()
            .filter_map(|(index, count)| (*count == 0).then_some(index))
            .collect();
        Self {
            remaining,
            dependents,
            ready,
            built: 0,
            reused: 0,
            active: 0,
            completed: 0,
            used_cpus: 0,
            errors: BTreeMap::new(),
        }
    }

    fn record(&mut self, index: usize, cpus: NonZeroUsize, result: Result<NodeOutcome>) {
        self.active -= 1;
        self.used_cpus -= cpus.get();
        self.completed += 1;
        match result {
            Ok(NodeOutcome::Built) => self.built += 1,
            Ok(NodeOutcome::Reused) => self.reused += 1,
            Err(error) => {
                self.errors.insert(index, error);
                return;
            }
        }
        for dependent in &self.dependents[index] {
            self.remaining[*dependent] -= 1;
            if self.remaining[*dependent] == 0 {
                self.ready.insert(*dependent);
            }
        }
    }

    fn started(&mut self, cpus: NonZeroUsize) {
        self.active += 1;
        self.used_cpus += cpus.get();
    }
}

// --- Scheduler execution ---

fn execute(graph: &BuildGraph, executor: &Executor, workers: usize) -> Result<GraphResult> {
    let mut scheduler = Scheduler::new(graph);
    run_scheduler(graph, executor, workers, &mut scheduler);
    if let Some((_, error)) = scheduler.errors.pop_first() {
        return Err(error);
    }
    if scheduler.completed != graph.nodes.len() {
        return Err(std::io::Error::other(format!(
            "build scheduler stopped after {} of {} nodes",
            scheduler.completed,
            graph.nodes.len()
        ))
        .into());
    }
    Ok(GraphResult {
        built: scheduler.built,
        reused: scheduler.reused,
        root_references: node_references(&graph.nodes[graph.root]),
    })
}

fn run_scheduler(
    graph: &BuildGraph,
    executor: &Executor,
    workers: usize,
    scheduler: &mut Scheduler,
) {
    let (sender, receiver) = mpsc::channel();
    std::thread::scope(|scope| loop {
        while scheduler.errors.is_empty()
            && scheduler.active < workers
            && scheduler.used_cpus < executor.cpu_count.get()
            && !scheduler.ready.is_empty()
        {
            let available = executor.cpu_count.get() - scheduler.used_cpus;
            let launch_count = scheduler
                .ready
                .len()
                .min(workers - scheduler.active)
                .min(available);
            for slot in 0..launch_count {
                let index = scheduler
                    .ready
                    .pop_first()
                    .expect("launch count follows ready nodes");
                let cpus = NonZeroUsize::new(
                    available / launch_count + usize::from(slot < available % launch_count),
                )
                .expect("each launched worker receives a CPU");
                let sender = sender.clone();
                let mut job = executor.clone();
                job.cpu_count = cpus;
                let node = &graph.nodes[index];
                scheduler.started(cpus);
                scope.spawn(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        execute_node(node, index, &job)
                    }))
                    .unwrap_or_else(|_| Err(std::io::Error::other("build worker panicked").into()));
                    let _ = sender.send((index, cpus, result));
                });
            }
        }
        if scheduler.active == 0 {
            break;
        }
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok((index, cpus, result)) => scheduler.record(index, cpus, result),
            Err(mpsc::RecvTimeoutError::Timeout) => executor.reporter.tick(),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    });
}

#[cfg(test)]
#[path = "driver_tests.rs"]
mod tests;
