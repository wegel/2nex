//! Per-job output counters, timing samples, and progress bars.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use indicatif::{HumanBytes, HumanDuration, MultiProgress, ProgressBar, ProgressStyle};

use super::estimate::{estimate, AttemptState, Estimate};
use super::{BuildPass, OutputMode, OutputOptions};
use crate::profile::{BuildProfile, ProfileRecorder};

// --- Job state ---

pub(crate) struct JobProgress {
    identity: JobIdentity,
    profile: Option<BuildProfile>,
    bar: Option<ProgressBar>,
    mode: OutputMode,
    live: LiveState,
    recording: Option<Recording>,
}

struct JobIdentity {
    label: String,
    attempts: usize,
    created: Instant,
}

struct LiveState {
    attempt: AtomicUsize,
    bytes: AtomicU64,
    estimate: Mutex<AttemptState>,
}

struct Recording {
    start: OnceLock<Instant>,
    recorder: Mutex<ProfileRecorder>,
    result: OnceLock<Vec<String>>,
}

// --- Output and timing ---

impl JobProgress {
    pub(super) fn new(
        label: &str,
        check: bool,
        profile: Option<BuildProfile>,
        dynamic: bool,
        multi: &MultiProgress,
        options: OutputOptions,
    ) -> Self {
        let bar = dynamic.then(|| job_bar(multi, label, profile.is_some()));
        Self {
            identity: JobIdentity {
                label: label.to_owned(),
                attempts: 1 + usize::from(check),
                created: Instant::now(),
            },
            profile,
            bar,
            mode: options.mode,
            live: LiveState {
                attempt: AtomicUsize::new(0),
                bytes: AtomicU64::new(0),
                estimate: Mutex::new(AttemptState::default()),
            },
            recording: options.record_profile.then(|| Recording {
                start: OnceLock::new(),
                recorder: Mutex::new(ProfileRecorder::new()),
                result: OnceLock::new(),
            }),
        }
    }

    pub(crate) fn begin_pass(&self, pass: BuildPass) {
        let attempt = pass.index();
        self.live.attempt.store(attempt, Ordering::Relaxed);
        self.live.bytes.store(0, Ordering::Relaxed);
        let now = Instant::now();
        self.live
            .estimate
            .lock()
            .expect("attempt progress lock")
            .begin(now);
        if pass == BuildPass::Primary {
            if let Some(recording) = &self.recording {
                let _ = recording.start.set(now);
            }
        }
    }

    pub(crate) fn output(&self, bytes: &[u8]) {
        if self.bar.is_none() && !self.mode.is_verbose() && self.recording.is_none() {
            return;
        }
        let total = self
            .live
            .bytes
            .fetch_add(bytes.len() as u64, Ordering::Relaxed)
            + bytes.len() as u64;
        self.record_sample(total);
        if self.mode.is_verbose() {
            self.print_output(bytes);
        }
    }

    pub(crate) fn end_pass(&self, pass: BuildPass) {
        if pass != BuildPass::Primary {
            return;
        }
        let Some(recording) = &self.recording else {
            return;
        };
        let Some(start) = recording.start.get() else {
            return;
        };
        let profile = recording
            .recorder
            .lock()
            .expect("profile recorder lock")
            .finish(self.live.bytes.load(Ordering::Relaxed), start.elapsed());
        let _ = recording.result.set(profile);
    }

    pub(crate) fn recorded_profile(&self) -> Option<Vec<String>> {
        self.recording.as_ref()?.result.get().cloned()
    }

    pub(super) fn tick(&self) {
        let Some(bar) = &self.bar else {
            return;
        };
        let bytes = self.live.bytes.load(Ordering::Relaxed);
        let attempt = self.live.attempt.load(Ordering::Relaxed);
        let mut state = self.live.estimate.lock().expect("attempt progress lock");
        let Some(started) = state.started() else {
            bar.set_message("preparing");
            return;
        };
        let elapsed = started.elapsed();
        if let Some(profile) = &self.profile {
            let estimate = estimate(
                profile,
                &mut state,
                bytes,
                attempt,
                self.identity.attempts,
                elapsed,
            );
            bar.set_position(estimate.percent.round() as u64);
            bar.set_message(estimate_message(
                estimate,
                bytes,
                attempt,
                self.identity.attempts,
            ));
        } else {
            self.tick_spinner(bar, bytes, attempt, elapsed);
        }
    }

    pub(super) fn finish(&self, success: bool) {
        let elapsed = HumanDuration(self.identity.created.elapsed());
        if let Some(bar) = &self.bar {
            if success {
                bar.set_position(100);
                bar.finish_with_message(format!("done in {elapsed}"));
            } else {
                bar.abandon_with_message(format!("failed after {elapsed}"));
            }
        } else if !self.mode.is_quiet() {
            let state = if success { "finished" } else { "failed" };
            eprintln!("{state} {} in {elapsed}", self.identity.label);
        }
    }

    // --- Private helpers ---

    fn record_sample(&self, total: u64) {
        if self.live.attempt.load(Ordering::Relaxed) != BuildPass::Primary.index() {
            return;
        }
        if let Some(recording) = &self.recording {
            let Some(start) = recording.start.get() else {
                return;
            };
            recording
                .recorder
                .lock()
                .expect("profile recorder lock")
                .record(total, start.elapsed());
        }
    }

    fn print_output(&self, bytes: &[u8]) {
        for line in bytes.split_inclusive(|byte| *byte == b'\n') {
            let line = String::from_utf8_lossy(line);
            let line = line.trim_end_matches(['\r', '\n']);
            if let Some(bar) = &self.bar {
                bar.println(format!("{} | {line}", self.identity.label));
            } else {
                eprintln!("{} | {line}", self.identity.label);
            }
        }
    }

    fn tick_spinner(
        &self,
        bar: &ProgressBar,
        bytes: u64,
        attempt: usize,
        elapsed: std::time::Duration,
    ) {
        bar.tick();
        bar.set_message(format!(
            "{} · {} · build {}/{}",
            HumanDuration(elapsed),
            HumanBytes(bytes),
            attempt + 1,
            self.identity.attempts
        ));
    }
}

// --- Drawing helpers ---

fn estimate_message(estimate: Estimate, bytes: u64, attempt: usize, attempts: usize) -> String {
    let eta = estimate.eta.map_or_else(
        || "finishing".to_owned(),
        |eta| format!("ETA {}", HumanDuration(eta)),
    );
    format!(
        "{eta} · {} · build {}/{}",
        HumanBytes(bytes),
        attempt + 1,
        attempts
    )
}

fn job_bar(multi: &MultiProgress, label: &str, known: bool) -> ProgressBar {
    let bar = if known {
        ProgressBar::new(100)
    } else {
        ProgressBar::new_spinner()
    };
    let template = if known {
        "{prefix:.cyan} [{bar:30.cyan/blue}] {pos:>3}% {msg}"
    } else {
        "{prefix:.cyan} {spinner:.green} {msg}"
    };
    bar.set_style(
        ProgressStyle::with_template(template)
            .expect("valid job progress style")
            .progress_chars("=>-"),
    );
    bar.set_prefix(label.to_owned());
    multi.add(bar)
}

#[cfg(test)]
#[path = "progress_job_tests.rs"]
mod tests;
