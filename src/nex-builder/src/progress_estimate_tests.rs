//! Tests for ETA calculations.

use std::time::Duration;

use super::{estimate, AttemptState};
use crate::profile::BuildProfile;

#[test]
fn estimates_one_build_from_recorded_wall_time() {
    let profile = profile(&["100:1000"]);
    let estimate = estimate(
        &profile,
        &mut AttemptState::default(),
        50,
        0,
        1,
        Duration::from_millis(500),
    );

    assert!((estimate.percent - 50.0).abs() < 0.1);
    assert_eq!(estimate.eta.expect("ETA").as_millis(), 500);
}

#[test]
fn calibration_uses_observed_checkpoint_time() {
    let profile = profile(&["100:1000", "200:2000"]);
    let estimate = estimate(
        &profile,
        &mut AttemptState::default(),
        100,
        0,
        1,
        Duration::from_secs(2),
    );

    assert!((estimate.percent - 50.0).abs() < 0.1);
    assert_eq!(estimate.eta.expect("ETA").as_secs(), 2);
}

#[test]
fn reproducibility_check_spans_two_builds() {
    let profile = profile(&["100:1000"]);
    let first = estimate(
        &profile,
        &mut AttemptState::default(),
        50,
        0,
        2,
        Duration::from_millis(500),
    );
    let second = estimate(
        &profile,
        &mut AttemptState::default(),
        50,
        1,
        2,
        Duration::from_millis(500),
    );

    assert!((first.percent - 25.0).abs() < 0.1);
    assert!((second.percent - 75.0).abs() < 0.1);
}

fn profile(values: &[&str]) -> BuildProfile {
    BuildProfile::parse(
        &values
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
    )
    .expect("parse profile")
}
