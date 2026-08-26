//! Tests for output defaults.

use super::{OutputMode, OutputOptions};

#[test]
fn default_output_uses_progress_without_manifest_writes() {
    assert_eq!(
        OutputOptions::default(),
        OutputOptions {
            mode: OutputMode::Progress,
            record_profile: false,
        }
    );
}
