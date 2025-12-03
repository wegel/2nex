//! progress display: renders progress bars using indicatif

use std::sync::Arc;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::tracker::ProgressState;

/// display mode for progress output
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    /// show only progress bar, no stdout
    ProgressBar,
    /// show stdout AND progress bar
    Verbose,
}

/// handles rendering progress to the terminal
#[derive(Clone)]
pub struct ProgressDisplay {
    mode: DisplayMode,
    bar: ProgressBar,
    package_name: String,
}

impl ProgressDisplay {
    /// create a new display (standalone, not part of MultiProgress)
    pub fn new(mode: DisplayMode, has_profile: bool, package_name: &str) -> Self {
        let bar = Self::create_bar(has_profile);
        bar.set_prefix(package_name.to_string());

        Self {
            mode,
            bar,
            package_name: package_name.to_string(),
        }
    }

    /// create a new display attached to a MultiProgress (for parallel builds)
    pub fn new_multi(
        mode: DisplayMode,
        has_profile: bool,
        package_name: &str,
        multi: &Arc<MultiProgress>,
    ) -> Self {
        let bar = Self::create_bar(has_profile);
        bar.set_prefix(package_name.to_string());
        let bar = multi.add(bar);

        Self {
            mode,
            bar,
            package_name: package_name.to_string(),
        }
    }

    fn create_bar(has_profile: bool) -> ProgressBar {
        if has_profile {
            let pb = ProgressBar::new(100);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{prefix:.cyan} [{bar:30.cyan/blue}] {pos:>3}% {msg}")
                    .expect("valid template")
                    .progress_chars("=>-"),
            );
            pb
        } else {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{prefix:.cyan} {spinner:.green} {msg}")
                    .expect("valid template"),
            );
            pb
        }
    }

    /// update display with current progress state
    pub fn update(&self, state: &ProgressState) {
        match state {
            ProgressState::Indeterminate { bytes } => {
                self.bar.tick();
                self.bar
                    .set_message(format!("{} bytes", format_bytes(*bytes)));
            }
            ProgressState::Tracking {
                percent,
                eta_secs,
                bytes,
                confidence,
            } => {
                self.bar.set_position(*percent as u64);

                let eta_str = format_eta(*eta_secs);
                let confidence_indicator = if *confidence < 0.3 {
                    "~"
                } else if *confidence < 0.6 {
                    ""
                } else {
                    ""
                };

                self.bar.set_message(format!(
                    "ETA: {}{} ({})",
                    confidence_indicator,
                    eta_str,
                    format_bytes(*bytes)
                ));
            }
            ProgressState::Stale { bytes } => {
                // switch to spinner style for stale profiles
                self.bar.set_style(
                    ProgressStyle::default_spinner()
                        .template("{prefix:.yellow} {spinner:.yellow} {msg}")
                        .expect("valid template"),
                );
                self.bar.tick();
                self.bar
                    .set_message(format!("(profile stale) {}", format_bytes(*bytes)));
            }
        }
    }

    /// print a line of stdout (only in verbose mode)
    pub fn print_stdout(&self, line: &str) {
        if self.mode == DisplayMode::Verbose {
            self.bar.suspend(|| println!("{}", line));
        }
    }

    /// mark progress as finished successfully
    pub fn finish(&self) {
        self.bar.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.green} [=============================] done")
                .expect("valid template"),
        );
        self.bar.finish();
    }

    /// mark progress as finished with error
    pub fn finish_with_error(&self, msg: &str) {
        self.bar.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.red} {msg}")
                .expect("valid template"),
        );
        self.bar.finish_with_message(format!("failed: {}", msg));
    }

    /// get the underlying progress bar (for advanced operations)
    pub fn bar(&self) -> &ProgressBar {
        &self.bar
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }
}

/// format bytes in human-readable form
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// format ETA in human-readable form
fn format_eta(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else if secs < 3600.0 {
        let mins = (secs / 60.0).floor();
        let remaining_secs = secs % 60.0;
        format!("{:.0}m {:.0}s", mins, remaining_secs)
    } else {
        let hours = (secs / 3600.0).floor();
        let remaining_mins = ((secs % 3600.0) / 60.0).floor();
        format!("{:.0}h {:.0}m", hours, remaining_mins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_works() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn format_eta_works() {
        assert_eq!(format_eta(30.0), "30s");
        assert_eq!(format_eta(90.0), "1m 30s");
        assert_eq!(format_eta(3700.0), "1h 1m");
    }
}
