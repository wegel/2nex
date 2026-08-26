//! Typed package references and their manifest paths.

use std::fmt;
use std::io;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::manifest::PackageManifest;
use crate::schema::{invalid, validate_name, validate_version};
use crate::{Error, Result as BuildResult};

const PACKAGE_PREFIX: &str = "x86_64/pkg";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum InputRef {
    Stored(String),
    Package(PackageRef),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct PackageKey {
    pub namespace: String,
    pub slug: String,
    pub version: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum RefKind {
    Output(String),
    Bundle(String),
    Files,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct PackageRef {
    pub key: PackageKey,
    pub kind: RefKind,
}

impl InputRef {
    pub(crate) fn parse(value: &str) -> BuildResult<Self> {
        if is_hash(value) {
            Ok(Self::Stored(value.to_owned()))
        } else {
            PackageRef::parse(value).map(Self::Package)
        }
    }
}

impl PackageRef {
    pub(crate) fn new(key: PackageKey, kind: RefKind) -> Self {
        Self { key, kind }
    }

    pub(crate) fn from_manifest(manifest: &PackageManifest, kind: RefKind) -> Self {
        Self::new(PackageKey::from_manifest(manifest), kind)
    }

    pub(crate) fn parse(value: &str) -> BuildResult<Self> {
        let Some(reference) = value
            .strip_prefix(PACKAGE_PREFIX)
            .and_then(|rest| rest.strip_prefix('/'))
        else {
            return Err(invalid_reference(value));
        };
        let malformed = || invalid_reference(value);
        let (package, kind) = if let Some(package) = reference.strip_suffix("/files") {
            (package, RefKind::Files)
        } else if let Some((package, member)) = reference.rsplit_once("/outputs/") {
            (package, RefKind::Output(member.to_owned()))
        } else if let Some((package, member)) = reference.rsplit_once("/bundles/") {
            (package, RefKind::Bundle(member.to_owned()))
        } else {
            return Err(malformed());
        };
        let (package, version) = package.rsplit_once('/').ok_or_else(malformed)?;
        let (namespace, slug) = package.rsplit_once('/').ok_or_else(malformed)?;
        for part in namespace.split('/') {
            validate_name(part, "namespace").map_err(|_| invalid_reference(value))?;
        }
        validate_name(slug, "package slug").map_err(|_| invalid_reference(value))?;
        validate_version(version).map_err(|_| invalid_reference(value))?;
        if let RefKind::Output(member) | RefKind::Bundle(member) = &kind {
            validate_name(member, "reference member").map_err(|_| invalid_reference(value))?;
        }
        Ok(Self {
            key: PackageKey {
                namespace: namespace.to_owned(),
                slug: slug.to_owned(),
                version: version.to_owned(),
            },
            kind,
        })
    }

    pub fn validate_manifest(&self, manifest: &PackageManifest) -> io::Result<()> {
        let package = &manifest.package;
        if package.namespace.trim_start_matches("pkg/") != self.key.namespace
            || package.slug != self.key.slug
            || package.version != self.key.version
        {
            return Err(invalid(format!(
                "reference {} does not match package {}/{}/{}",
                self, package.namespace, package.slug, package.version
            )));
        }
        let exists = match self.kind {
            RefKind::Output(ref member) => {
                member != "discard" && manifest.outputs.contains_key(member)
            }
            RefKind::Bundle(ref member) => manifest.bundles.contains_key(member),
            RefKind::Files => true,
        };
        if exists {
            Ok(())
        } else {
            Err(invalid(format!(
                "reference {} names an output the package does not publish",
                self
            )))
        }
    }
}

impl PackageKey {
    pub(crate) fn from_manifest(manifest: &PackageManifest) -> Self {
        Self {
            namespace: manifest
                .package
                .namespace
                .trim_start_matches("pkg/")
                .to_owned(),
            slug: manifest.package.slug.clone(),
            version: manifest.package.version.clone(),
        }
    }
}

impl fmt::Display for InputRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stored(hash) => formatter.write_str(hash),
            Self::Package(reference) => reference.fmt(formatter),
        }
    }
}

impl fmt::Display for PackageKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}/{}/{}",
            self.namespace, self.slug, self.version
        )
    }
}

impl fmt::Display for PackageRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{PACKAGE_PREFIX}/{}", self.key)?;
        match &self.kind {
            RefKind::Output(name) => write!(formatter, "/outputs/{name}"),
            RefKind::Bundle(name) => write!(formatter, "/bundles/{name}"),
            RefKind::Files => formatter.write_str("/files"),
        }
    }
}

impl<'de> Deserialize<'de> for InputRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(D::Error::custom)
    }
}

impl Serialize for InputRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

fn is_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn invalid_reference(value: &str) -> Error {
    Error::InvalidReference {
        value: value.to_owned(),
    }
}

#[cfg(test)]
#[path = "reference_tests.rs"]
mod tests;
