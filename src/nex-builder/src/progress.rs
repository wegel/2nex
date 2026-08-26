//! Output policy and terminal progress components.

#[path = "progress_estimate.rs"]
mod estimate;
#[path = "progress_job.rs"]
mod job;
#[path = "progress_reporter.rs"]
mod reporter;

pub(crate) use job::JobProgress;
pub(crate) use reporter::Reporter;

#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(usize)]
pub(crate) enum BuildPass {
    Primary,
    Reproducibility,
}

impl BuildPass {
    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

// --- Public options ---

/// Complete terminal behavior for one graph build.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OutputMode {
    /// Draw progress bars while keeping build-script output in log files.
    #[default]
    Progress,
    /// Draw progress bars and prefix build-script output with its node.
    ProgressVerbose,
    /// Print stable lifecycle lines without redrawing the terminal.
    Plain,
    /// Print lifecycle lines and prefixed build-script output.
    PlainVerbose,
    /// Write no status or build-script output.
    Quiet,
}

/// Controls terminal output and timing-profile recording.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OutputOptions {
    /// Selects one valid terminal behavior.
    pub mode: OutputMode,
    /// Replace each successfully built manifest's timing profile.
    pub record_profile: bool,
}

impl OutputOptions {
    /// Options suitable for callers that do not want terminal output.
    pub fn quiet() -> Self {
        Self {
            mode: OutputMode::Quiet,
            record_profile: false,
        }
    }
}

impl OutputMode {
    pub(crate) fn draws_progress(self) -> bool {
        matches!(self, Self::Progress | Self::ProgressVerbose)
    }

    pub(crate) fn is_verbose(self) -> bool {
        matches!(self, Self::ProgressVerbose | Self::PlainVerbose)
    }

    pub(crate) fn is_quiet(self) -> bool {
        self == Self::Quiet
    }
}

#[cfg(test)]
#[path = "progress_tests.rs"]
mod tests;
