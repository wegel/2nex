use std::fs;
use std::io;
use serde_yaml::Value;

use super::types::*;

pub fn detect_manifest_kind(doc: &Value) -> ManifestKind {
    if let Some(kind) = doc.get("kind").and_then(|v| v.as_str()) {
        if kind.eq_ignore_ascii_case("system") {
            return ManifestKind::System;
        }
    }
    if doc.get("system").is_some() && doc.get("package").is_none() {
        ManifestKind::System
    } else {
        ManifestKind::Package
    }
}

pub fn validate_system_manifest(manifest: &SystemManifest) -> io::Result<()> {
    if manifest.packages.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "System manifests must specify at least one entry under 'packages'",
        ));
    }
    if manifest.system.version.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "System manifests must set system.version",
        ));
    }
    Ok(())
}

pub fn load_manifest(file_path: &str) -> io::Result<ManifestData> {
    let manifest_str = fs::read_to_string(file_path)?;
    let doc: Value = serde_yaml::from_str(&manifest_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    match detect_manifest_kind(&doc) {
        ManifestKind::Package => {
            let manifest: Manifest = serde_yaml::from_value(doc)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Ok(ManifestData::Package(manifest))
        }
        ManifestKind::System => {
            let sys: SystemManifest = serde_yaml::from_value(doc)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            validate_system_manifest(&sys)?;
            Ok(ManifestData::System(sys))
        }
    }
}
