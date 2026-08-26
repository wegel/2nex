//! Graph-wide terminal state and lifecycle events.

use std::collections::BTreeMap;
use std::io::{self, IsTerminal};
use std::sync::{Arc, Mutex};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::{JobProgress, OutputOptions};
use crate::profile::BuildProfile;

// --- Reporter state ---

pub(crate) struct Reporter {
    options: OutputOptions,
    multi: MultiProgress,
    overall: Option<ProgressBar>,
    active: Mutex<BTreeMap<usize, Arc<JobProgress>>>,
}

// --- Lifecycle events ---

impl Reporter {
    pub(crate) fn new(options: OutputOptions, total: usize, workers: usize, cpus: usize) -> Self {
        let dynamic = options.mode.draws_progress() && io::stderr().is_terminal();
        let multi = MultiProgress::new();
        let reporter = Self {
            options,
            overall: dynamic.then(|| overall_bar(&multi, total)),
            multi,
            active: Mutex::new(BTreeMap::new()),
        };
        if !options.mode.is_quiet() && !dynamic {
            reporter.line(format!(
                "graph: {total} nodes, {workers} workers, {cpus} CPUs"
            ));
        }
        reporter
    }

    pub(crate) fn options(&self) -> OutputOptions {
        self.options
    }

    pub(crate) fn start(
        &self,
        index: usize,
        label: &str,
        profile_items: &[String],
        check: bool,
    ) -> Arc<JobProgress> {
        let profile = self.parse_profile(label, profile_items);
        let job = Arc::new(JobProgress::new(
            label,
            check,
            profile,
            self.overall.is_some(),
            &self.multi,
            self.options,
        ));
        if self.overall.is_some() {
            self.active
                .lock()
                .expect("progress state lock")
                .insert(index, Arc::clone(&job));
        } else if !self.options.mode.is_quiet() {
            self.line(format!("building {label}"));
        }
        job
    }

    pub(crate) fn reused(&self, label: &str) {
        if !self.options.mode.is_quiet() {
            self.line(format!("reusing {label}"));
        }
        self.advance_overall();
    }

    pub(crate) fn finished(&self, index: usize, job: &JobProgress) {
        self.complete(index);
        job.finish(true);
        self.advance_overall();
    }

    pub(crate) fn failed(&self, index: usize, job: &JobProgress) {
        self.complete(index);
        job.finish(false);
        self.advance_overall();
    }

    pub(crate) fn tick(&self) {
        let active = self.active.lock().expect("progress state lock");
        for job in active.values() {
            job.tick();
        }
    }

    pub(crate) fn finish_graph(&self, success: bool) {
        let Some(overall) = &self.overall else {
            return;
        };
        if success {
            overall.finish_and_clear();
        } else {
            overall.abandon_with_message("graph failed");
        }
    }

    // --- Private helpers ---

    fn parse_profile(&self, label: &str, items: &[String]) -> Option<BuildProfile> {
        match BuildProfile::parse(items) {
            Ok(profile) => Some(profile),
            Err(_) if items.is_empty() => None,
            Err(error) => {
                self.line(format!("warning: {label}: {error}"));
                None
            }
        }
    }

    fn complete(&self, index: usize) {
        self.active
            .lock()
            .expect("progress state lock")
            .remove(&index);
    }

    fn advance_overall(&self) {
        if let Some(overall) = &self.overall {
            overall.inc(1);
        }
    }

    fn line(&self, line: String) {
        if self.options.mode.is_quiet() {
            return;
        }
        if self.overall.is_some() {
            let _ = self.multi.println(line);
        } else {
            eprintln!("{line}");
        }
    }
}

fn overall_bar(multi: &MultiProgress, total: usize) -> ProgressBar {
    let bar = ProgressBar::new(total as u64);
    bar.set_style(
        ProgressStyle::with_template("graph [{bar:30.green/black}] {pos}/{len}")
            .expect("valid overall progress style")
            .progress_chars("=>-"),
    );
    multi.add(bar)
}
