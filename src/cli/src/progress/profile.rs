//! build profile: recorded checkpoint data for progress estimation

use std::fmt;

/// a checkpoint in the build profile: (bytes_seen, elapsed_ms)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub bytes: u64,
    pub time_ms: u64,
}

#[derive(Debug)]
pub enum ProfileError {
    InvalidFormat(String),
    Empty,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProfileError::InvalidFormat(msg) => write!(f, "invalid profile format: {}", msg),
            ProfileError::Empty => write!(f, "profile has no checkpoints"),
        }
    }
}

impl std::error::Error for ProfileError {}

/// parsed build profile - checkpoints from a previous build
#[derive(Debug, Clone)]
pub struct BuildProfile {
    pub checkpoints: Vec<Checkpoint>, // last one is the total
}

impl BuildProfile {
    /// parse from YAML array of "bytes:time_ms" strings
    pub fn parse(items: &[String]) -> Result<Self, ProfileError> {
        if items.is_empty() {
            return Err(ProfileError::Empty);
        }

        let mut checkpoints = Vec::new();
        for cp_str in items {
            let cp_parts: Vec<&str> = cp_str.split(':').collect();
            if cp_parts.len() != 2 {
                return Err(ProfileError::InvalidFormat(format!(
                    "checkpoint must be bytes:time_ms, got: {}",
                    cp_str
                )));
            }
            let bytes: u64 = cp_parts[0].parse().map_err(|_| {
                ProfileError::InvalidFormat(format!("invalid bytes in checkpoint: {}", cp_str))
            })?;
            let time_ms: u64 = cp_parts[1].parse().map_err(|_| {
                ProfileError::InvalidFormat(format!("invalid time_ms in checkpoint: {}", cp_str))
            })?;
            checkpoints.push(Checkpoint { bytes, time_ms });
        }

        Ok(BuildProfile { checkpoints })
    }

    /// serialize to YAML array of "bytes:time_ms" strings
    pub fn serialize(&self) -> Vec<String> {
        self.checkpoints
            .iter()
            .map(|cp| format!("{}:{}", cp.bytes, cp.time_ms))
            .collect()
    }

    /// total bytes (from last checkpoint)
    pub fn total_bytes(&self) -> u64 {
        self.checkpoints.last().map(|cp| cp.bytes).unwrap_or(0)
    }

    /// total time in ms (from last checkpoint)
    pub fn total_time_ms(&self) -> u64 {
        self.checkpoints.last().map(|cp| cp.time_ms).unwrap_or(0)
    }

    /// find the checkpoint just before or at the given byte offset
    /// returns None if bytes is before all checkpoints
    pub fn checkpoint_at_or_before(&self, bytes: u64) -> Option<&Checkpoint> {
        let mut result = None;
        for cp in &self.checkpoints {
            if cp.bytes <= bytes {
                result = Some(cp);
            } else {
                break;
            }
        }
        result
    }

    /// find the checkpoint just after the given byte offset
    /// returns None if bytes is past all checkpoints
    pub fn checkpoint_after(&self, bytes: u64) -> Option<&Checkpoint> {
        for cp in &self.checkpoints {
            if cp.bytes > bytes {
                return Some(cp);
            }
        }
        None
    }

    /// check if the profile seems valid for the observed byte count
    /// (within tolerance of total_bytes)
    pub fn is_valid_for_bytes(&self, observed: u64, tolerance: f64) -> bool {
        let expected = self.total_bytes() as f64;
        if expected == 0.0 {
            return true;
        }
        let diff = (observed as f64 - expected).abs() / expected;
        diff <= tolerance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_serialize_roundtrip() {
        let items = s(&["40000:5200", "95000:14800", "245890:34500"]);
        let profile = BuildProfile::parse(&items).unwrap();
        assert_eq!(profile.checkpoints.len(), 3);
        assert_eq!(profile.total_bytes(), 245890);
        assert_eq!(profile.total_time_ms(), 34500);
        assert_eq!(profile.serialize(), items);
    }

    #[test]
    fn parse_single_checkpoint() {
        let items = s(&["1000:500"]);
        let profile = BuildProfile::parse(&items).unwrap();
        assert_eq!(profile.checkpoints.len(), 1);
        assert_eq!(profile.total_bytes(), 1000);
        assert_eq!(profile.total_time_ms(), 500);
    }

    #[test]
    fn parse_empty_fails() {
        assert!(BuildProfile::parse(&[]).is_err());
    }

    #[test]
    fn checkpoint_lookup() {
        let profile = BuildProfile::parse(&s(&["1000:100", "2000:200", "3000:300"])).unwrap();

        // before first checkpoint
        assert!(profile.checkpoint_at_or_before(500).is_none());
        assert_eq!(profile.checkpoint_after(500).unwrap().bytes, 1000);

        // at first checkpoint
        assert_eq!(profile.checkpoint_at_or_before(1000).unwrap().bytes, 1000);
        assert_eq!(profile.checkpoint_after(1000).unwrap().bytes, 2000);

        // between checkpoints
        assert_eq!(profile.checkpoint_at_or_before(1500).unwrap().bytes, 1000);
        assert_eq!(profile.checkpoint_after(1500).unwrap().bytes, 2000);

        // at last checkpoint
        assert_eq!(profile.checkpoint_at_or_before(3000).unwrap().bytes, 3000);
        assert!(profile.checkpoint_after(3000).is_none());

        // past last checkpoint
        assert_eq!(profile.checkpoint_at_or_before(5000).unwrap().bytes, 3000);
        assert!(profile.checkpoint_after(5000).is_none());
    }

    #[test]
    fn validity_check() {
        let profile = BuildProfile::parse(&s(&["1000:500"])).unwrap();

        // within 20% tolerance
        assert!(profile.is_valid_for_bytes(1000, 0.2)); // exact
        assert!(profile.is_valid_for_bytes(1100, 0.2)); // 10% over
        assert!(profile.is_valid_for_bytes(900, 0.2)); // 10% under
        assert!(profile.is_valid_for_bytes(1200, 0.2)); // 20% over

        // outside tolerance
        assert!(!profile.is_valid_for_bytes(1300, 0.2)); // 30% over
        assert!(!profile.is_valid_for_bytes(700, 0.2)); // 30% under
    }
}
