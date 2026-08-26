//! Bounded build-timing profiles and targeted manifest updates.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use crate::schema::invalid;

const MAX_SAMPLES: usize = 128;
const PROFILE_POINTS: &[u64] = &[2, 8, 18, 30, 40, 50, 60, 75, 90, 100];

// --- Profile data ---

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Checkpoint {
    pub bytes: u64,
    pub time_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BuildProfile {
    checkpoints: Vec<Checkpoint>,
}

impl BuildProfile {
    pub(crate) fn parse(items: &[String]) -> Result<Self, String> {
        if items.is_empty() {
            return Err("profile is empty".to_owned());
        }
        let mut checkpoints: Vec<Checkpoint> = Vec::with_capacity(items.len());
        for item in items {
            let (bytes, time_ms) = item
                .split_once(':')
                .ok_or_else(|| format!("profile checkpoint {item:?} is not bytes:milliseconds"))?;
            let checkpoint = Checkpoint {
                bytes: parse_number(bytes, item)?,
                time_ms: parse_number(time_ms, item)?,
            };
            if let Some(previous) = checkpoints.last() {
                let reverses =
                    checkpoint.bytes < previous.bytes || checkpoint.time_ms < previous.time_ms;
                if reverses || checkpoint == *previous {
                    return Err(format!("profile checkpoint {item:?} is not monotonic"));
                }
            }
            checkpoints.push(checkpoint);
        }
        Ok(Self { checkpoints })
    }

    pub(crate) fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    pub(crate) fn total_time_ms(&self) -> u64 {
        self.checkpoints
            .last()
            .map_or(0, |checkpoint| checkpoint.time_ms)
    }
}

fn parse_number(value: &str, checkpoint: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|_| format!("profile checkpoint {checkpoint:?} contains a non-number"))
}

// --- Bounded recording ---

#[derive(Debug)]
pub(crate) struct ProfileRecorder {
    samples: Vec<Checkpoint>,
    interval: u64,
    next_sample: u64,
}

impl ProfileRecorder {
    pub(crate) fn new() -> Self {
        Self {
            samples: Vec::new(),
            interval: 4096,
            next_sample: 4096,
        }
    }

    pub(crate) fn record(&mut self, bytes: u64, elapsed: Duration) {
        if bytes < self.next_sample {
            return;
        }
        self.samples.push(Checkpoint {
            bytes,
            time_ms: millis(elapsed),
        });
        if self.samples.len() == MAX_SAMPLES {
            self.samples = self.samples.iter().copied().step_by(2).collect();
            self.interval = self.interval.saturating_mul(2);
        }
        self.next_sample = bytes.saturating_add(self.interval);
    }

    pub(crate) fn finish(&self, bytes: u64, elapsed: Duration) -> Vec<String> {
        let final_point = Checkpoint {
            bytes,
            time_ms: millis(elapsed),
        };
        let mut points = Vec::with_capacity(PROFILE_POINTS.len());
        for percentage in PROFILE_POINTS {
            let target = bytes.saturating_mul(*percentage) / 100;
            let point = if *percentage == 100 {
                final_point
            } else {
                self.samples
                    .iter()
                    .copied()
                    .find(|sample| sample.bytes >= target)
                    .unwrap_or(final_point)
            };
            if points.last().is_none_or(|previous: &Checkpoint| {
                point.bytes > previous.bytes && point.time_ms >= previous.time_ms
            }) {
                points.push(point);
            }
        }
        if points.last().is_none_or(|point| *point != final_point) {
            points.push(final_point);
        }
        points
            .into_iter()
            .map(|point| format!("{}:{}", point.bytes, point.time_ms))
            .collect()
    }
}

fn millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

// --- Targeted manifest updates ---

pub(crate) fn update_manifest(path: &Path, profile: &[String]) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    let trailing_newline = source.ends_with('\n');
    let mut lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
    replace_profile_line(&mut lines, profile)?;
    let mut updated = lines.join("\n");
    if trailing_newline {
        updated.push('\n');
    }
    atomic_write(path, updated.as_bytes())
}

fn replace_profile_line(lines: &mut Vec<String>, profile: &[String]) -> io::Result<()> {
    let build = lines
        .iter()
        .position(|line| line == "build:")
        .ok_or_else(|| invalid("manifest has no top-level build field"))?;
    let end = lines
        .iter()
        .enumerate()
        .skip(build + 1)
        .find_map(|(index, line)| is_top_level_field(line).then_some(index))
        .unwrap_or(lines.len());
    let script = lines[build + 1..end]
        .iter()
        .position(|line| line.trim_start().starts_with("script:"))
        .map(|offset| build + 1 + offset)
        .ok_or_else(|| invalid("build field has no script"))?;
    let indent = lines[script]
        .chars()
        .take_while(|character| character.is_whitespace())
        .collect::<String>();
    let value = format!("{indent}profile: [{}]", profile.join(", "));
    let existing = lines[build + 1..end]
        .iter()
        .position(|line| line.trim_start().starts_with("profile:"))
        .map(|offset| build + 1 + offset);
    if let Some(index) = existing {
        let multiline = lines[index].trim() == "profile:";
        lines[index] = value;
        if multiline {
            while lines
                .get(index + 1)
                .is_some_and(|line| line.trim().is_empty() || line.trim_start().starts_with("- "))
            {
                lines.remove(index + 1);
            }
        }
    } else {
        lines.insert(script, value);
    }
    Ok(())
}

fn is_top_level_field(line: &str) -> bool {
    !line.is_empty()
        && !line.starts_with(char::is_whitespace)
        && !line.trim_start().starts_with('#')
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| invalid("manifest path has no parent"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary
        .as_file()
        .set_permissions(metadata.permissions())?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map(|_| ())
        .map_err(|error| error.error)
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
