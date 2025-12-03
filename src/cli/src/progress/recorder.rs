//! profile recorder: samples timing data during builds

use std::time::Instant;

use super::profile::{BuildProfile, Checkpoint};

/// records timing samples during a build for later playback
pub struct ProfileRecorder {
    start_time: Instant,
    samples: Vec<(u64, u64)>, // (bytes, time_ms)
    last_sample_bytes: u64,
    sample_interval_bytes: u64,
}

impl ProfileRecorder {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            samples: Vec::new(),
            last_sample_bytes: 0,
            sample_interval_bytes: 4096, // sample every 4KB
        }
    }

    /// record bytes received - samples at intervals
    pub fn record_bytes(&mut self, total_bytes: u64) {
        if total_bytes >= self.last_sample_bytes + self.sample_interval_bytes {
            let elapsed = self.start_time.elapsed().as_millis() as u64;
            self.samples.push((total_bytes, elapsed));
            self.last_sample_bytes = total_bytes;
        }
    }

    /// finalize and produce a profile with selected checkpoints
    pub fn finalize(self, total_bytes: u64) -> BuildProfile {
        let total_time = self.start_time.elapsed().as_millis() as u64;
        let checkpoints = self.select_checkpoints(total_bytes, total_time);

        BuildProfile { checkpoints }
    }

    /// select representative checkpoints from samples
    /// weighted-early distribution: more checkpoints early for faster calibration
    fn select_checkpoints(&self, total_bytes: u64, total_time: u64) -> Vec<Checkpoint> {
        if self.samples.is_empty() {
            return vec![Checkpoint {
                bytes: total_bytes,
                time_ms: total_time,
            }];
        }

        // weighted-early: 3 checkpoints in first 18% for fast calibration on long builds
        const PERCENTAGES: &[u64] = &[2, 8, 18, 30, 40, 50, 60, 75, 90, 100];

        let mut checkpoints = Vec::with_capacity(PERCENTAGES.len());
        let mut sample_idx = 0;

        for &pct in PERCENTAGES {
            let target_bytes = if pct == 100 {
                total_bytes
            } else {
                total_bytes * pct / 100
            };

            // find first sample at or past target_bytes
            while sample_idx < self.samples.len() && self.samples[sample_idx].0 < target_bytes {
                sample_idx += 1;
            }

            if pct == 100 {
                // always use exact final values
                checkpoints.push(Checkpoint {
                    bytes: total_bytes,
                    time_ms: total_time,
                });
            } else if sample_idx < self.samples.len() {
                let (bytes, time_ms) = self.samples[sample_idx];
                checkpoints.push(Checkpoint { bytes, time_ms });
                sample_idx += 1; // don't reuse this sample
            }
        }

        checkpoints
    }
}

impl Default for ProfileRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_samples() {
        let mut recorder = ProfileRecorder::new();

        // simulate receiving bytes
        for i in 0..100 {
            recorder.record_bytes((i + 1) * 1000);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        let profile = recorder.finalize(100_000);

        // should have checkpoints
        assert!(!profile.checkpoints.is_empty());
        // last checkpoint should be the total
        assert_eq!(profile.total_bytes(), 100_000);
        // should not exceed target count (plus some margin)
        assert!(profile.checkpoints.len() <= 12);
    }

    #[test]
    fn handles_no_samples() {
        let recorder = ProfileRecorder::new();
        let profile = recorder.finalize(1000);

        assert_eq!(profile.checkpoints.len(), 1);
        assert_eq!(profile.total_bytes(), 1000);
    }

    #[test]
    fn handles_few_samples() {
        let mut recorder = ProfileRecorder::new();
        recorder.record_bytes(5000);
        recorder.record_bytes(10000);

        let profile = recorder.finalize(15000);

        // should include all samples plus final
        assert!(profile.checkpoints.len() <= 3);
        assert_eq!(profile.total_bytes(), 15000);
    }
}
