//! LUKS2 binary header parsing
//!
//! parses the 4096-byte binary header at the start of a LUKS2 volume.

use alloc::string::String;
use core::fmt;

/// LUKS2 magic bytes at offset 0
pub const LUKS_MAGIC: [u8; 6] = *b"LUKS\xba\xbe";
pub const LUKS2_VERSION: u16 = 2;
pub const BINARY_HEADER_SIZE: usize = 4096;

/// LUKS2 binary header (first 512 bytes are used, rest is padding)
#[derive(Debug)]
#[allow(dead_code)]
pub struct Luks2BinaryHeader {
    pub version: u16,
    pub hdr_size: u64,
    pub seqid: u64,
    pub label: String,
    pub checksum_alg: String,
    pub salt: [u8; 64],
    pub uuid: String,
    pub hdr_offset: u64,
}

#[derive(Debug)]
pub enum HeaderError {
    InvalidMagic,
    UnsupportedVersion(u16),
    BufferTooSmall,
}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeaderError::InvalidMagic => write!(f, "invalid LUKS magic"),
            HeaderError::UnsupportedVersion(v) => write!(f, "unsupported LUKS version: {}", v),
            HeaderError::BufferTooSmall => write!(f, "buffer too small for header"),
        }
    }
}

impl Luks2BinaryHeader {
    /// parse LUKS2 binary header from raw bytes
    pub fn parse(data: &[u8]) -> Result<Self, HeaderError> {
        if data.len() < 512 {
            return Err(HeaderError::BufferTooSmall);
        }

        // check magic
        if data[0..6] != LUKS_MAGIC {
            return Err(HeaderError::InvalidMagic);
        }

        // check version (big-endian u16 at offset 6)
        let version = u16::from_be_bytes([data[6], data[7]]);
        if version != LUKS2_VERSION {
            return Err(HeaderError::UnsupportedVersion(version));
        }

        // parse header size (big-endian u64 at offset 8)
        let hdr_size = u64::from_be_bytes(data[8..16].try_into().unwrap());

        // parse sequence id (big-endian u64 at offset 16)
        let seqid = u64::from_be_bytes(data[16..24].try_into().unwrap());

        // parse label (null-terminated string at offset 24, 48 bytes)
        let label = parse_null_terminated(&data[24..72]);

        // parse checksum algorithm (null-terminated at offset 72, 32 bytes)
        let checksum_alg = parse_null_terminated(&data[72..104]);

        // parse salt (64 bytes at offset 104)
        let mut salt = [0u8; 64];
        salt.copy_from_slice(&data[104..168]);

        // parse UUID (null-terminated at offset 168, 40 bytes)
        let uuid = parse_null_terminated(&data[168..208]);

        // parse header offset (big-endian u64 at offset 256)
        let hdr_offset = u64::from_be_bytes(data[256..264].try_into().unwrap());

        Ok(Self {
            version,
            hdr_size,
            seqid,
            label,
            checksum_alg,
            salt,
            uuid,
            hdr_offset,
        })
    }

    /// offset where JSON metadata begins (after binary header)
    pub fn json_offset(&self) -> u64 {
        BINARY_HEADER_SIZE as u64
    }

    /// size of JSON metadata area
    pub fn json_size(&self) -> u64 {
        // JSON area extends from end of binary header to hdr_size
        // but there's also a secondary header, so actual JSON is hdr_size/2 - 4096
        // for simplicity, we read up to hdr_size - 4096
        self.hdr_size.saturating_sub(BINARY_HEADER_SIZE as u64)
    }
}

/// parse null-terminated string from byte slice
fn parse_null_terminated(data: &[u8]) -> String {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).into_owned()
}

/// check if data starts with LUKS magic
#[allow(dead_code)]
pub fn is_luks(data: &[u8]) -> bool {
    data.len() >= 6 && data[0..6] == LUKS_MAGIC
}

/// check if data is LUKS2 specifically
pub fn is_luks2(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    if data[0..6] != LUKS_MAGIC {
        return false;
    }
    let version = u16::from_be_bytes([data[6], data[7]]);
    version == LUKS2_VERSION
}
