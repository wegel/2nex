//! progress tracker: uses recorded profile to estimate build progress
//! includes self-tuning calibration based on checkpoint comparisons
//!
//! key design: progress is estimated from TIME, not just bytes.
//! bytes are used for calibration, but the display interpolates smoothly
//! based on elapsed time even when no new bytes arrive.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use super::profile::BuildProfile;

/// current state of progress tracking
#[derive(Debug, Clone)]
pub enum ProgressState {
    /// no profile - can't estimate, show spinner
    Indeterminate { bytes: u64 },
    /// actively tracking with estimates (including before first checkpoint)
    Tracking {
        percent: f64,
        eta_secs: f64,
        bytes: u64,
        confidence: f64, // 0.0-1.0, increases as more checkpoints are passed
    },
    /// profile diverged too much from reality
    Stale { bytes: u64 },
}

/// mutable calibration state protected by mutex
struct CalibrationState {
    speed_factor: f64,
    passed_checkpoints: usize,
    stale: bool,
}

/// tracks progress during a build using a recorded profile
/// thread-safe: can be updated from reader thread and polled from display thread
pub struct ProgressTracker {
    profile: BuildProfile,
    start_time: Instant,
    bytes_seen: AtomicU64,
    calibration: Mutex<CalibrationState>,
    parallel_jobs: u32,
}

impl ProgressTracker {
    pub fn new(profile: BuildProfile, parallel_jobs: u32) -> Self {
        Self {
            profile,
            start_time: Instant::now(),
            bytes_seen: AtomicU64::new(0),
            calibration: Mutex::new(CalibrationState {
                speed_factor: 1.0,
                passed_checkpoints: 0,
                stale: false,
            }),
            parallel_jobs: parallel_jobs.max(1),
        }
    }

    /// record new bytes (called from reader thread)
    pub fn add_bytes(&self, additional_bytes: u64) {
        self.bytes_seen
            .fetch_add(additional_bytes, Ordering::Relaxed);
        self.update_calibration();
    }

    /// get current progress state based on elapsed time (can be called anytime)
    /// this is the key method for smooth progress - it interpolates based on time
    pub fn current_state(&self) -> ProgressState {
        let bytes = self.bytes_seen.load(Ordering::Relaxed);
        let cal = self.calibration.lock().unwrap();

        if cal.stale {
            return ProgressState::Stale { bytes };
        }

        let percent = self.compute_percent_time_based(&cal);
        let eta_secs = self.estimate_eta(&cal);
        let confidence = self.compute_confidence(&cal);

        ProgressState::Tracking {
            percent,
            eta_secs,
            bytes,
            confidence,
        }
    }

    /// legacy method for compatibility - calls add_bytes then current_state
    pub fn update(&self, additional_bytes: u64) -> ProgressState {
        self.add_bytes(additional_bytes);
        self.current_state()
    }

    /// compute progress percentage based purely on TIME
    /// bytes are used only for calibration, not for progress display
    /// this ensures smooth, consistent progress regardless of output bursts
    fn compute_percent_time_based(&self, cal: &CalibrationState) -> f64 {
        let total_time_ms = self.profile.total_time_ms();
        if total_time_ms == 0 {
            return 0.0;
        }

        let elapsed_ms = self.start_time.elapsed().as_millis() as f64;

        // expected time adjusted by speed factor and parallelism
        let expected_total_ms = total_time_ms as f64 * cal.speed_factor * self.parallel_jobs as f64;

        // pure time-based progress - smooth and consistent
        (elapsed_ms / expected_total_ms * 100.0).min(99.9)
    }

    /// update calibration based on checkpoints we've passed
    fn update_calibration(&self) {
        let total_checkpoints = self.profile.checkpoints.len();
        if total_checkpoints == 0 {
            return;
        }

        let bytes = self.bytes_seen.load(Ordering::Relaxed);
        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;

        let mut cal = self.calibration.lock().unwrap();

        // check each checkpoint we might have passed
        for (idx, cp) in self.profile.checkpoints.iter().enumerate() {
            if idx < cal.passed_checkpoints {
                continue;
            }

            if bytes >= cp.bytes {
                if cp.time_ms > 0 {
                    let observed_factor = elapsed_ms as f64 / cp.time_ms as f64;
                    let adjusted_factor = observed_factor / self.parallel_jobs as f64;

                    // blend with previous estimate
                    let weight = 0.6;
                    cal.speed_factor = cal.speed_factor * (1.0 - weight) + adjusted_factor * weight;

                    // mark stale only if we're running much slower than expected
                    // (5x slower suggests the profile is from a very different machine/scenario)
                    // running faster is fine - it just means this machine is faster
                    if adjusted_factor > 5.0 {
                        cal.stale = true;
                    }
                }
                cal.passed_checkpoints = idx + 1;
            } else {
                break;
            }
        }
    }

    /// estimate remaining time in seconds (purely time-based for consistency)
    fn estimate_eta(&self, cal: &CalibrationState) -> f64 {
        let total_time_ms = self.profile.total_time_ms();
        if total_time_ms == 0 {
            return 0.0;
        }

        let elapsed_ms = self.start_time.elapsed().as_millis() as f64;
        let expected_total_ms = total_time_ms as f64 * cal.speed_factor * self.parallel_jobs as f64;

        // pure time-based ETA - smooth countdown
        let remaining_ms = (expected_total_ms - elapsed_ms).max(0.0);
        remaining_ms / 1000.0
    }

    /// compute confidence in our estimates (0.0-1.0)
    fn compute_confidence(&self, cal: &CalibrationState) -> f64 {
        let total_checkpoints = self.profile.checkpoints.len();
        if total_checkpoints == 0 {
            return 0.1; // minimal confidence with no checkpoints
        }

        // base confidence from checkpoints passed
        let checkpoint_confidence =
            (cal.passed_checkpoints as f64 / total_checkpoints as f64).min(1.0);

        // boost from elapsed time (we get more confident as time passes)
        let elapsed_ms = self.start_time.elapsed().as_millis() as f64;
        let total_time_ms = self.profile.total_time_ms() as f64;
        let time_confidence = if total_time_ms > 0.0 {
            (elapsed_ms / total_time_ms).min(1.0)
        } else {
            0.0
        };

        // before first checkpoint, still give some confidence based on time
        if cal.passed_checkpoints == 0 {
            return (time_confidence * 0.3).max(0.1); // 10-30% confidence before first checkpoint
        }

        // combine checkpoint and time confidence
        (checkpoint_confidence * 0.7 + time_confidence * 0.3).min(1.0)
    }

    pub fn is_stale(&self) -> bool {
        self.calibration.lock().unwrap().stale
    }

    pub fn bytes_seen(&self) -> u64 {
        self.bytes_seen.load(Ordering::Relaxed)
    }

    pub fn speed_factor(&self) -> f64 {
        self.calibration.lock().unwrap().speed_factor
    }

    pub fn passed_checkpoints(&self) -> usize {
        self.calibration.lock().unwrap().passed_checkpoints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_profile(checkpoints: &[(u64, u64)]) -> BuildProfile {
        use super::super::profile::Checkpoint;
        BuildProfile {
            checkpoints: checkpoints
                .iter()
                .map(|&(bytes, time_ms)| Checkpoint { bytes, time_ms })
                .collect(),
        }
    }

    #[test]
    fn tracks_from_start() {
        let profile = make_profile(&[(1000, 100), (2000, 200)]);
        let tracker = ProgressTracker::new(profile, 1);

        // even before any bytes, should give time-based estimate
        let state = tracker.current_state();
        assert!(matches!(state, ProgressState::Tracking { .. }));
    }

    #[test]
    fn tracks_after_first_checkpoint() {
        let profile = make_profile(&[(1000, 100), (2000, 200)]);
        let tracker = ProgressTracker::new(profile, 1);

        tracker.add_bytes(1100);
        let state = tracker.current_state();
        assert!(matches!(state, ProgressState::Tracking { .. }));

        if let ProgressState::Tracking { confidence, .. } = state {
            assert!(confidence > 0.3); // higher confidence after checkpoint
        }
    }

    #[test]
    fn calibrates_speed_factor() {
        let profile = make_profile(&[(1000, 1000), (2000, 2000)]);
        let tracker = ProgressTracker::new(profile, 1);

        std::thread::sleep(std::time::Duration::from_millis(500));
        tracker.add_bytes(1000);

        // speed factor should have been adjusted
        assert!(tracker.passed_checkpoints() > 0);
    }

    #[test]
    fn adjusts_for_parallelism() {
        let profile = make_profile(&[(1000, 1000)]);
        let tracker1 = ProgressTracker::new(profile.clone(), 1);
        let tracker4 = ProgressTracker::new(profile, 4);

        assert_eq!(tracker1.parallel_jobs, 1);
        assert_eq!(tracker4.parallel_jobs, 4);
    }

    #[test]
    fn thread_safe_updates() {
        use std::sync::Arc;
        use std::thread;

        let profile = make_profile(&[(10000, 1000), (20000, 2000)]);
        let tracker = Arc::new(ProgressTracker::new(profile, 1));

        let tracker_writer = Arc::clone(&tracker);
        let writer = thread::spawn(move || {
            for _ in 0..100 {
                tracker_writer.add_bytes(100);
                thread::sleep(std::time::Duration::from_millis(1));
            }
        });

        let tracker_reader = Arc::clone(&tracker);
        let reader = thread::spawn(move || {
            for _ in 0..50 {
                let _ = tracker_reader.current_state();
                thread::sleep(std::time::Duration::from_millis(2));
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();

        assert_eq!(tracker.bytes_seen(), 10000);
    }
}
