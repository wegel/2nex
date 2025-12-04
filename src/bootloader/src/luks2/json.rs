//! LUKS2 JSON metadata parsing
//!
//! parses the JSON metadata area containing keyslots, segments, and digests.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use serde::Deserialize;

/// top-level LUKS2 JSON metadata
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Luks2Metadata {
    pub keyslots: BTreeMap<String, Keyslot>,
    pub segments: BTreeMap<String, Segment>,
    pub digests: BTreeMap<String, Digest>,
    #[serde(default)]
    pub config: Config,
}

/// keyslot entry - stores encrypted master key material
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Keyslot {
    #[serde(rename = "type")]
    pub keyslot_type: String,
    pub key_size: u32,
    #[serde(default)]
    pub priority: Option<i32>,
    pub kdf: Kdf,
    pub af: AntiForensic,
    pub area: KeyslotArea,
}

/// key derivation function parameters
#[derive(Debug, Deserialize)]
pub struct Kdf {
    #[serde(rename = "type")]
    pub kdf_type: String,
    #[serde(deserialize_with = "de_base64")]
    pub salt: Vec<u8>,
    #[serde(default)]
    pub time: Option<u32>,
    #[serde(default)]
    pub memory: Option<u32>,
    #[serde(default)]
    pub cpus: Option<u32>,
    #[serde(default)]
    pub iterations: Option<u32>,
    #[serde(default)]
    pub hash: Option<String>,
}

/// anti-forensic splitter parameters
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AntiForensic {
    #[serde(rename = "type")]
    pub af_type: String,
    pub stripes: u32,
    pub hash: String,
}

/// keyslot storage area on disk
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct KeyslotArea {
    #[serde(rename = "type")]
    pub area_type: String,
    #[serde(deserialize_with = "de_u64_string")]
    pub offset: u64,
    #[serde(deserialize_with = "de_u64_string")]
    pub size: u64,
    pub encryption: String,
    pub key_size: u32,
}

/// segment describing encrypted data area
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Segment {
    #[serde(rename = "type")]
    pub segment_type: String,
    #[serde(deserialize_with = "de_u64_string")]
    pub offset: u64,
    #[serde(deserialize_with = "de_size_or_dynamic")]
    pub size: SegmentSize,
    pub encryption: String,
    pub sector_size: u32,
    #[serde(default)]
    pub iv_tweak: u64,
}

/// segment size can be a number or "dynamic"
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SegmentSize {
    Fixed(u64),
    Dynamic,
}

/// digest for verifying master key
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Digest {
    #[serde(rename = "type")]
    pub digest_type: String,
    pub keyslots: Vec<String>,
    pub segments: Vec<String>,
    #[serde(deserialize_with = "de_base64")]
    pub salt: Vec<u8>,
    #[serde(deserialize_with = "de_base64")]
    pub digest: Vec<u8>,
    pub hash: String,
    pub iterations: u32,
}

/// config section
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
pub struct Config {
    #[serde(default)]
    pub json_size: Option<String>,
    #[serde(default)]
    pub keyslots_size: Option<String>,
}

#[derive(Debug)]
pub enum JsonError {
    ParseError(String),
    InvalidUtf8,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::ParseError(msg) => write!(f, "JSON parse error: {}", msg),
            JsonError::InvalidUtf8 => write!(f, "invalid UTF-8 in JSON"),
        }
    }
}

impl Luks2Metadata {
    /// parse LUKS2 JSON metadata from bytes
    pub fn parse(data: &[u8]) -> Result<Self, JsonError> {
        // find null terminator (JSON is null-terminated in LUKS2)
        let json_end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        let json_str =
            core::str::from_utf8(&data[..json_end]).map_err(|_| JsonError::InvalidUtf8)?;

        serde_json::from_str(json_str).map_err(|e| JsonError::ParseError(alloc::format!("{}", e)))
    }

    /// find first active keyslot with argon2id KDF
    pub fn find_argon2id_keyslot(&self) -> Option<(&String, &Keyslot)> {
        self.keyslots
            .iter()
            .find(|(_, ks)| ks.kdf.kdf_type == "argon2id" || ks.kdf.kdf_type == "argon2i")
    }

    /// find first PBKDF2 keyslot (fallback)
    pub fn find_pbkdf2_keyslot(&self) -> Option<(&String, &Keyslot)> {
        self.keyslots
            .iter()
            .find(|(_, ks)| ks.kdf.kdf_type == "pbkdf2")
    }

    /// get data segment (usually "0")
    pub fn data_segment(&self) -> Option<&Segment> {
        self.segments.get("0")
    }

    /// find digest that covers a keyslot
    pub fn find_digest_for_keyslot(&self, keyslot_id: &str) -> Option<&Digest> {
        self.digests
            .values()
            .find(|d| d.keyslots.iter().any(|k| k == keyslot_id))
    }
}

// custom deserializers

fn de_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use base64::prelude::*;
    use serde::de::Error;

    let s = String::deserialize(deserializer)?;
    BASE64_STANDARD
        .decode(&s)
        .map_err(|e| Error::custom(alloc::format!("base64 decode error: {}", e)))
}

fn de_u64_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let s = String::deserialize(deserializer)?;
    s.parse::<u64>()
        .map_err(|e| Error::custom(alloc::format!("u64 parse error: {}", e)))
}

fn de_size_or_dynamic<'de, D>(deserializer: D) -> Result<SegmentSize, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s == "dynamic" {
        Ok(SegmentSize::Dynamic)
    } else {
        s.parse::<u64>()
            .map(SegmentSize::Fixed)
            .map_err(serde::de::Error::custom)
    }
}
