//! build progress estimation system
//!
//! records stdout byte offsets and timing data during builds,
//! then uses that profile to show accurate progress bars on subsequent builds.
//!
//! ## self-tuning calibration
//!
//! rather than requiring external calibration, the system automatically
//! calibrates by comparing actual elapsed time at each checkpoint to the
//! recorded time. this works across different machines without configuration.
//!
//! ## usage
//!
//! first build (no profile):
//! - show indeterminate spinner
//! - record checkpoints as build progresses
//! - save profile to manifest on success
//!
//! subsequent builds:
//! - parse existing profile from manifest
//! - track progress using checkpoints
//! - self-calibrate as checkpoints are passed
//! - show accurate ETA based on profile + observed rate

pub mod display;
pub mod profile;
pub mod recorder;
pub mod tracker;

pub use display::{DisplayMode, ProgressDisplay};
pub use profile::{BuildProfile, Checkpoint, ProfileError};
pub use recorder::ProfileRecorder;
pub use tracker::{ProgressState, ProgressTracker};

use std::io::{self, BufRead, BufReader, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use indicatif::MultiProgress;

/// configuration for progress tracking during a build
#[derive(Clone)]
pub struct BuildProgressConfig {
    /// package name for display
    pub package_name: String,
    /// show stdout in addition to progress bar
    pub verbose: bool,
    /// existing profile from manifest (each element is "bytes:time_ms")
    pub profile: Vec<String>,
    /// force re-recording even if profile exists
    pub record_profile: bool,
    /// number of parallel jobs (for calibration adjustment)
    pub parallel_jobs: u32,
    /// shared MultiProgress for parallel builds (None for single builds)
    pub multi_progress: Option<Arc<MultiProgress>>,
}

impl BuildProgressConfig {
    pub fn new(package_name: &str) -> Self {
        Self {
            package_name: package_name.to_string(),
            verbose: false,
            profile: Vec::new(),
            record_profile: false,
            parallel_jobs: 1,
            multi_progress: None,
        }
    }
}

impl Default for BuildProgressConfig {
    fn default() -> Self {
        Self {
            package_name: "build".to_string(),
            verbose: false,
            profile: Vec::new(),
            record_profile: false,
            parallel_jobs: 1,
            multi_progress: None,
        }
    }
}

/// result of running a build with progress tracking
pub struct ProgressResult {
    /// new profile if one was recorded (each element is "bytes:time_ms")
    pub new_profile: Option<Vec<String>>,
    /// total bytes of output processed
    pub total_bytes: u64,
    /// captured output for error display (only populated if not verbose)
    pub captured_output: Option<Vec<u8>>,
}

/// run a build with progress tracking, consuming combined stdout+stderr
///
/// returns the new profile if one was recorded (only when record_profile is true)
pub fn run_with_progress<R: Read + Send + 'static>(
    output: R,
    config: &BuildProgressConfig,
) -> io::Result<ProgressResult> {
    let should_record = config.record_profile;
    let package_name = &config.package_name;

    let display_mode = if config.verbose {
        DisplayMode::Verbose
    } else {
        DisplayMode::ProgressBar
    };

    // if no profile exists, show indeterminate progress (spinner)
    // only actually record if --record-profile was requested
    if config.profile.is_empty() || should_record {
        run_recording(
            output,
            display_mode,
            package_name,
            &config.multi_progress,
            should_record,
        )
    } else {
        // playback mode - use existing profile
        match BuildProfile::parse(&config.profile) {
            Ok(profile) => run_playback(
                output,
                profile,
                display_mode,
                package_name,
                config.parallel_jobs,
                &config.multi_progress,
            ),
            Err(e) => {
                eprintln!(
                    "warning: invalid profile, showing indeterminate progress: {}",
                    e
                );
                run_recording(
                    output,
                    display_mode,
                    package_name,
                    &config.multi_progress,
                    false,
                )
            }
        }
    }
}

fn run_recording<R: Read + Send + 'static>(
    stdout: R,
    mode: DisplayMode,
    package_name: &str,
    multi: &Option<Arc<MultiProgress>>,
    save_profile: bool,
) -> io::Result<ProgressResult> {
    let display = match multi {
        Some(m) => ProgressDisplay::new_multi(mode, false, package_name, m),
        None => ProgressDisplay::new(mode, false, package_name),
    };

    let mut recorder = ProfileRecorder::new();
    let mut reader = BufReader::new(stdout);
    let mut total_bytes = 0u64;
    let mut line_buf = Vec::new();
    let capture = !matches!(mode, DisplayMode::Verbose);
    let mut captured: Vec<u8> = Vec::new();

    // read lines as bytes to handle non-UTF-8 output (eg progress bars with \r)
    loop {
        line_buf.clear();
        let bytes_read = reader.read_until(b'\n', &mut line_buf)?;
        if bytes_read == 0 {
            break;
        }
        total_bytes += bytes_read as u64;

        if capture {
            captured.extend_from_slice(&line_buf);
        }

        // convert to string lossily (replaces invalid UTF-8 with replacement char)
        let line = String::from_utf8_lossy(&line_buf);
        let line = line.trim_end_matches('\n').trim_end_matches('\r');

        recorder.record_bytes(total_bytes);
        display.update(&ProgressState::Indeterminate { bytes: total_bytes });
        display.print_stdout(line);
    }

    display.finish();

    // only return profile if explicitly requested
    let new_profile = if save_profile {
        Some(recorder.finalize(total_bytes).serialize())
    } else {
        None
    };

    Ok(ProgressResult {
        new_profile,
        total_bytes,
        captured_output: if capture { Some(captured) } else { None },
    })
}

fn run_playback<R: Read + Send + 'static>(
    stdout: R,
    profile: BuildProfile,
    mode: DisplayMode,
    package_name: &str,
    parallel_jobs: u32,
    multi: &Option<Arc<MultiProgress>>,
) -> io::Result<ProgressResult> {
    let display = match multi {
        Some(m) => ProgressDisplay::new_multi(mode, true, package_name, m),
        None => ProgressDisplay::new(mode, true, package_name),
    };

    let tracker = Arc::new(ProgressTracker::new(profile, parallel_jobs));
    let done = Arc::new(AtomicBool::new(false));

    // spawn display updater thread - updates every 100ms for smooth progress
    let display_tracker = Arc::clone(&tracker);
    let display_done = Arc::clone(&done);
    let display_thread = {
        let display = display.clone();
        thread::spawn(move || {
            while !display_done.load(Ordering::Relaxed) {
                let state = display_tracker.current_state();
                display.update(&state);
                thread::sleep(Duration::from_millis(100));
            }
        })
    };

    // read output in main thread (as bytes to handle non-UTF-8)
    let mut reader = BufReader::new(stdout);
    let mut total_bytes = 0u64;
    let mut line_buf = Vec::new();
    let capture = !matches!(mode, DisplayMode::Verbose);
    let mut captured: Vec<u8> = Vec::new();

    loop {
        line_buf.clear();
        let bytes_read = reader.read_until(b'\n', &mut line_buf)?;
        if bytes_read == 0 {
            break;
        }
        total_bytes += bytes_read as u64;

        if capture {
            captured.extend_from_slice(&line_buf);
        }

        let line = String::from_utf8_lossy(&line_buf);
        let line = line.trim_end_matches('\n').trim_end_matches('\r');

        tracker.add_bytes(bytes_read as u64);
        display.print_stdout(line);
    }

    // signal display thread to stop
    done.store(true, Ordering::Relaxed);
    let _ = display_thread.join();

    display.finish();

    Ok(ProgressResult {
        new_profile: None,
        total_bytes,
        captured_output: if capture { Some(captured) } else { None },
    })
}
