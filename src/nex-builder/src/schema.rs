//! Shared manifest schema version and scalar checks.

use std::io;
use std::path::Path;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer};

use crate::reference::InputRef;
use crate::Error;

/// The only manifest schema understood by this builder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SchemaVersion;

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let version = u64::deserialize(deserializer)?;
        if version == 1 {
            Ok(Self)
        } else {
            Err(D::Error::custom(format!(
                "unsupported schema version {version}"
            )))
        }
    }
}

pub(crate) fn validate_name(name: &str, kind: &str) -> io::Result<()> {
    let safe = !matches!(name, "" | "." | "..")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if safe {
        Ok(())
    } else {
        Err(invalid(format!("invalid {kind} name {name:?}")))
    }
}

pub(crate) fn validate_version(version: &str) -> io::Result<()> {
    let valid = !version.is_empty()
        && version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+'));
    if valid {
        Ok(())
    } else {
        Err(invalid(format!("invalid version {version:?}")))
    }
}

pub(crate) fn validate_hash(hash: &str, kind: &str) -> io::Result<()> {
    if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(invalid(format!("invalid {kind} {hash:?}")))
    }
}

pub(crate) fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

pub(crate) fn invalid_data(error: serde_yaml::Error) -> io::Error {
    invalid(error.to_string())
}

pub(crate) fn manifest_error(
    path: &Path,
    bytes: &[u8],
    root: &str,
    source: serde_yaml::Error,
) -> Error {
    if let Ok(value) = serde_yaml::from_slice::<serde_yaml::Value>(bytes) {
        let version = value.get(root).and_then(|item| item.get("schema"));
        if let Some(version) = version.and_then(serde_yaml::Value::as_u64) {
            if version != 1 {
                return Error::UnsupportedSchema {
                    path: path.to_path_buf(),
                    version,
                };
            }
        }
        if let Some(error) = invalid_reference(&value) {
            return error;
        }
    }
    Error::Manifest {
        path: path.to_path_buf(),
        source,
    }
}

fn invalid_reference(value: &serde_yaml::Value) -> Option<Error> {
    for field in ["dependencies", "packages"] {
        if let Some(items) = value.get(field).and_then(serde_yaml::Value::as_sequence) {
            for reference in items
                .iter()
                .filter_map(|item| item.get("commit"))
                .filter_map(serde_yaml::Value::as_str)
            {
                if let Err(error) = InputRef::parse(reference) {
                    return Some(error);
                }
            }
        }
    }
    if let Some(providers) = value
        .get("providers")
        .and_then(serde_yaml::Value::as_mapping)
    {
        for reference in providers.values().filter_map(serde_yaml::Value::as_str) {
            if let Err(error) = InputRef::parse(reference) {
                return Some(error);
            }
        }
    }
    None
}
