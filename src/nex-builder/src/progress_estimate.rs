//! ETA math kept independent from terminal drawing.

use std::time::{Duration, Instant};

use crate::profile::BuildProfile;

#[derive(Default)]
pub(super) struct AttemptState {
    started: Option<Instant>,
    next_checkpoint: usize,
    scale: Option<f64>,
}

impl AttemptState {
    pub(super) fn begin(&mut self, now: Instant) {
        self.started = Some(now);
        self.next_checkpoint = 0;
    }

    pub(super) fn started(&self) -> Option<Instant> {
        self.started
    }
}

pub(super) struct Estimate {
    pub percent: f64,
    pub eta: Option<Duration>,
}

pub(super) fn estimate(
    profile: &BuildProfile,
    state: &mut AttemptState,
    bytes: u64,
    attempt: usize,
    attempts: usize,
    elapsed: Duration,
) -> Estimate {
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    calibrate(profile, state, bytes, elapsed_ms);
    let scale = state.scale.unwrap_or(1.0).clamp(0.05, 20.0);
    let expected_ms = profile.total_time_ms().max(1) as f64 * scale;
    let fraction = (elapsed_ms / expected_ms).clamp(0.0, 0.999);
    let percent = ((attempt as f64 + fraction) / attempts as f64 * 100.0).min(99.9);
    let future_ms = attempts.saturating_sub(attempt + 1) as f64 * expected_ms;
    let remaining_ms = (expected_ms - elapsed_ms).max(0.0) + future_ms;
    Estimate {
        percent,
        eta: (remaining_ms > 0.0).then(|| Duration::from_secs_f64(remaining_ms / 1000.0)),
    }
}

fn calibrate(profile: &BuildProfile, state: &mut AttemptState, bytes: u64, elapsed_ms: f64) {
    while let Some(checkpoint) = profile.checkpoints().get(state.next_checkpoint) {
        if checkpoint.bytes > bytes {
            break;
        }
        if checkpoint.time_ms > 0 {
            let observed = elapsed_ms / checkpoint.time_ms as f64;
            state.scale = Some(
                state
                    .scale
                    .map_or(observed, |scale| scale * 0.4 + observed * 0.6),
            );
        }
        state.next_checkpoint += 1;
    }
}

#[cfg(test)]
#[path = "progress_estimate_tests.rs"]
mod tests;
