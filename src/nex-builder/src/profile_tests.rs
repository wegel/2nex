//! Tests for bounded timing profiles and manifest edits.

use std::fs;
use std::time::Duration;

use tempfile::tempdir;

use super::{update_manifest, BuildProfile, ProfileRecorder, MAX_SAMPLES};

#[test]
fn parses_monotonic_profiles() {
    let items = strings(&["100:20", "200:35", "400:80"]);
    let profile = BuildProfile::parse(&items).expect("parse monotonic profile");

    assert_eq!(profile.total_time_ms(), 80);
    assert_eq!(profile.checkpoints().len(), 3);
}

#[test]
fn rejects_invalid_or_reversing_profiles() {
    assert!(BuildProfile::parse(&strings(&["invalid"])).is_err());
    assert!(BuildProfile::parse(&strings(&["200:20", "100:30"])).is_err());
    assert!(BuildProfile::parse(&strings(&["100:30", "200:20"])).is_err());
    assert!(BuildProfile::parse(&strings(&["100:20", "100:20"])).is_err());
    assert!(BuildProfile::parse(&strings(&["100:20", "100:30"])).is_ok());
}

#[test]
fn recorder_keeps_bounded_samples_and_exact_end() {
    let mut recorder = ProfileRecorder::new();
    for index in 1..10_000 {
        recorder.record(index * 4096, Duration::from_millis(index));
    }
    assert!(recorder.samples.len() < MAX_SAMPLES);

    let profile = recorder.finish(40_960_000, Duration::from_secs(10));
    assert!(profile.len() <= 10);
    assert_eq!(
        profile.last().expect("final profile sample"),
        "40960000:10000"
    );
}

#[test]
fn updates_inline_profile_without_reformatting_manifest() {
    let temporary = tempdir().expect("create temporary directory");
    let path = temporary.path().join("package.yaml");
    fs::write(
        &path,
        "package:\n  slug: test\nbuild:\n  environment: abc\n  profile: [1:2]\n\n  script: |\n    true\noutputs: {}\n",
    )
    .expect("write manifest fixture");

    update_manifest(&path, &strings(&["10:20", "30:40"])).expect("update existing profile");

    assert_eq!(
        fs::read_to_string(path).expect("read updated manifest"),
        "package:\n  slug: test\nbuild:\n  environment: abc\n  profile: [10:20, 30:40]\n\n  script: |\n    true\noutputs: {}\n"
    );
}

#[test]
fn inserts_missing_profile_before_script() {
    let temporary = tempdir().expect("create temporary directory");
    let path = temporary.path().join("package.yaml");
    fs::write(
        &path,
        "build:\n  environment: abc\n  script: |\n    true\noutputs: {}\n",
    )
    .expect("write manifest fixture");

    update_manifest(&path, &strings(&["10:20"])).expect("insert profile");

    assert_eq!(
        fs::read_to_string(path).expect("read updated manifest"),
        "build:\n  environment: abc\n  profile: [10:20]\n  script: |\n    true\noutputs: {}\n"
    );
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}
