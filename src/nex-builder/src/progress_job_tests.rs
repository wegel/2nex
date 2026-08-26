//! Tests for per-job timing-profile recording.

use indicatif::MultiProgress;

use super::JobProgress;
use crate::progress::{BuildPass, OutputMode, OutputOptions};

#[test]
fn records_only_the_first_reproducibility_build() {
    let job = JobProgress::new(
        "package tests/example/1.0",
        true,
        None,
        false,
        &MultiProgress::new(),
        OutputOptions {
            mode: OutputMode::Quiet,
            record_profile: true,
        },
    );
    job.begin_pass(BuildPass::Primary);
    job.output(b"first build\n");
    job.end_pass(BuildPass::Primary);
    let first = job.recorded_profile().expect("first build profile");

    job.begin_pass(BuildPass::Reproducibility);
    job.output(b"second build emits much more output than the first\n");
    job.end_pass(BuildPass::Reproducibility);

    assert_eq!(job.recorded_profile().expect("retained profile"), first);
    assert!(first.len() <= 10);
}
