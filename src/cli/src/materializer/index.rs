//! Provider priority helpers for the materializer.
//!
//! With precomputed deps, we don't need to build provider indices at runtime.
//! This module only provides phase priority helpers for any remaining use cases.

/// Get phase priority from commit path (higher = later phase = higher priority).
pub fn phase_priority(commit: &str) -> u32 {
    if commit.contains("/bootstrap/phase0/") {
        0
    } else if commit.contains("/bootstrap/phase1/") {
        1
    } else if commit.contains("/bootstrap/phase2/") {
        2
    } else if commit.contains("/bootstrap/phase3/") {
        3
    } else {
        // non-bootstrap packages have highest priority
        100
    }
}

#[cfg(test)]
mod tests {
    use super::phase_priority;

    #[test]
    fn test_phase_priority() {
        assert_eq!(
            phase_priority("x86_64/pkg/base/glibc/2.40/bootstrap/phase0/outputs/lib"),
            0
        );
        assert_eq!(
            phase_priority("x86_64/pkg/base/glibc/2.40/bootstrap/phase1/outputs/lib"),
            1
        );
        assert_eq!(
            phase_priority("x86_64/pkg/base/glibc/2.40/bootstrap/phase2/outputs/lib"),
            2
        );
        assert_eq!(
            phase_priority("x86_64/pkg/base/glibc/2.40/bootstrap/phase3/outputs/lib"),
            3
        );
        assert_eq!(
            phase_priority("x86_64/pkg/kernel/linux/6.12/outputs/bin"),
            100
        );
    }
}
