//! LUKS2 encrypted volume support
//!
//! detects and unlocks LUKS2 volumes, providing a decrypting block reader.

pub mod crypto;
pub mod header;
pub mod json;
pub mod reader;

use alloc::string::String;
use alloc::vec;
use core::fmt;

use crate::disk::RootPartition;
use crate::passphrase;
use crate::tpm;

pub use reader::DecryptingReader;

#[derive(Debug)]
pub enum Luks2Error {
    InvalidHeader(header::HeaderError),
    InvalidJson(json::JsonError),
    IoError(String),
    NoKeyslot,
    NoSegment,
    CryptoError(crypto::CryptoError),
    PassphraseError,
    KeyVerificationFailed,
    InvalidKeySize,
    UnsupportedCipher(String),
}

impl fmt::Display for Luks2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Luks2Error::InvalidHeader(e) => write!(f, "invalid header: {}", e),
            Luks2Error::InvalidJson(e) => write!(f, "invalid JSON: {}", e),
            Luks2Error::IoError(msg) => write!(f, "I/O error: {}", msg),
            Luks2Error::NoKeyslot => write!(f, "no usable keyslot found"),
            Luks2Error::NoSegment => write!(f, "no data segment found"),
            Luks2Error::CryptoError(e) => write!(f, "crypto error: {}", e),
            Luks2Error::PassphraseError => write!(f, "passphrase error"),
            Luks2Error::KeyVerificationFailed => write!(f, "key verification failed"),
            Luks2Error::InvalidKeySize => write!(f, "invalid key size"),
            Luks2Error::UnsupportedCipher(c) => write!(f, "unsupported cipher: {}", c),
        }
    }
}

impl From<header::HeaderError> for Luks2Error {
    fn from(e: header::HeaderError) -> Self {
        Luks2Error::InvalidHeader(e)
    }
}

impl From<json::JsonError> for Luks2Error {
    fn from(e: json::JsonError) -> Self {
        Luks2Error::InvalidJson(e)
    }
}

impl From<crypto::CryptoError> for Luks2Error {
    fn from(e: crypto::CryptoError) -> Self {
        Luks2Error::CryptoError(e)
    }
}

/// unlocked LUKS2 volume
pub struct Luks2Volume {
    pub reader: DecryptingReader,
}

/// detect if partition contains a LUKS2 header
pub fn is_luks2(partition: &RootPartition) -> Result<bool, Luks2Error> {
    let block_size = partition.block_size as usize;

    // read first block to check magic
    let mut buf = vec![0u8; block_size];
    partition
        .read_blocks(0, &mut buf)
        .map_err(|e| Luks2Error::IoError(alloc::format!("read: {:?}", e)))?;

    Ok(header::is_luks2(&buf))
}

/// unlock LUKS2 volume and return decrypting reader
pub fn unlock(partition: &RootPartition) -> Result<Luks2Volume, Luks2Error> {
    log::info!("luks2: reading header...");

    let block_size = partition.block_size as usize;

    // read binary header (at least 4096 bytes, align to block size)
    let header_blocks = (header::BINARY_HEADER_SIZE + block_size - 1) / block_size;
    let mut header_buf = vec![0u8; header_blocks * block_size];
    partition
        .read_blocks(0, &mut header_buf)
        .map_err(|e| Luks2Error::IoError(alloc::format!("read header: {:?}", e)))?;

    let bin_header = header::Luks2BinaryHeader::parse(&header_buf)?;

    log::info!(
        "luks2: version {}, uuid {}, header size {} bytes",
        bin_header.version,
        bin_header.uuid,
        bin_header.hdr_size
    );

    // read JSON metadata
    let json_offset = bin_header.json_offset();
    let json_size = bin_header.json_size() as usize;

    // limit JSON read to reasonable size (64KB should be enough)
    let json_read_size = core::cmp::min(json_size, 64 * 1024);
    let json_start_block = json_offset / block_size as u64;
    let json_blocks = (json_read_size + block_size - 1) / block_size;
    let mut json_buf = vec![0u8; json_blocks * block_size];

    partition
        .read_blocks(json_start_block, &mut json_buf)
        .map_err(|e| Luks2Error::IoError(alloc::format!("read JSON: {:?}", e)))?;

    let metadata = json::Luks2Metadata::parse(&json_buf[..json_read_size])?;

    // get data segment
    let segment = metadata.data_segment().ok_or(Luks2Error::NoSegment)?;

    // verify cipher is supported
    if !segment.encryption.contains("aes") || !segment.encryption.contains("xts") {
        return Err(Luks2Error::UnsupportedCipher(segment.encryption.clone()));
    }

    log::info!(
        "luks2: data segment at offset {}, sector size {}",
        segment.offset,
        segment.sector_size
    );

    // try TPM first, fall back to passphrase
    let master_key = if let Some(tpm_key) = tpm::try_unseal_master_key() {
        // TPM path: got master key directly, just verify it
        log::info!("luks2: using TPM-provided master key");
        verify_master_key_against_metadata(&tpm_key, &metadata)?;
        tpm_key
    } else {
        // passphrase path: derive key and decrypt keyslot
        log::info!("luks2: TPM unavailable, falling back to passphrase");
        recover_master_key_from_passphrase(partition, block_size, &metadata)?
    };

    // create decrypting reader
    let reader = DecryptingReader::new(
        partition.handle,
        partition.block_size,
        partition.start_lba,
        &master_key,
        segment.offset,
        segment.sector_size,
        segment.iv_tweak,
    )?;

    log::info!("luks2: volume unlocked successfully");

    Ok(Luks2Volume { reader })
}

/// verify master key against any available digest in metadata
fn verify_master_key_against_metadata(
    master_key: &[u8],
    metadata: &json::Luks2Metadata,
) -> Result<(), Luks2Error> {
    // find any digest that covers segment 0
    let digest = metadata
        .digests
        .values()
        .find(|d| d.segments.iter().any(|s| s == "0"));

    if let Some(digest) = digest {
        log::info!("luks2: verifying master key against digest...");
        let valid = crypto::verify_master_key(
            master_key,
            &digest.salt,
            &digest.digest,
            digest.iterations,
            &digest.hash,
        )?;

        if !valid {
            return Err(Luks2Error::KeyVerificationFailed);
        }
        log::info!("luks2: master key verified");
    }

    Ok(())
}

/// recover master key using passphrase + KDF + keyslot decryption
fn recover_master_key_from_passphrase(
    partition: &RootPartition,
    block_size: usize,
    metadata: &json::Luks2Metadata,
) -> Result<alloc::vec::Vec<u8>, Luks2Error> {
    // find usable keyslot (prefer argon2id, fall back to pbkdf2)
    let (slot_id, keyslot) = metadata
        .find_argon2id_keyslot()
        .or_else(|| metadata.find_pbkdf2_keyslot())
        .ok_or(Luks2Error::NoKeyslot)?;

    log::info!(
        "luks2: using keyslot {} with {} KDF",
        slot_id,
        keyslot.kdf.kdf_type
    );

    // get passphrase
    let passphrase = passphrase::read_passphrase("Enter LUKS passphrase: ")
        .map_err(|_| Luks2Error::PassphraseError)?;

    // derive key from passphrase
    log::info!("luks2: deriving key (this may take a while)...");
    let derived_key =
        crypto::derive_key(&passphrase, &keyslot.kdf, keyslot.area.key_size as usize)?;

    // read encrypted key material from keyslot area
    let area_offset = keyslot.area.offset;
    let area_size = keyslot.area.size as usize;

    let area_start_block = area_offset / block_size as u64;
    let area_blocks = (area_size + block_size - 1) / block_size;
    let mut area_buf = vec![0u8; area_blocks * block_size];

    partition
        .read_blocks(area_start_block, &mut area_buf)
        .map_err(|e| Luks2Error::IoError(alloc::format!("read keyslot area: {:?}", e)))?;

    let offset_in_block = (area_offset % block_size as u64) as usize;
    let key_material = &mut area_buf[offset_in_block..offset_in_block + area_size];

    // decrypt key material using derived key
    // keyslot area uses 512-byte sectors by default
    let keyslot_sector_size = 512;
    let keyslot_cipher = crypto::AesXts::new(&derived_key, keyslot_sector_size)?;
    keyslot_cipher.decrypt_sectors(0, key_material);

    // apply anti-forensic merge to recover master key
    let master_key = crypto::af_merge(
        key_material,
        keyslot.key_size as usize,
        keyslot.af.stripes,
        &keyslot.af.hash,
    )?;

    log::info!("luks2: recovered master key ({} bytes)", master_key.len());

    // verify master key against digest if available
    if let Some(digest) = metadata.find_digest_for_keyslot(slot_id) {
        log::info!("luks2: verifying master key...");
        let valid = crypto::verify_master_key(
            &master_key,
            &digest.salt,
            &digest.digest,
            digest.iterations,
            &digest.hash,
        )?;

        if !valid {
            return Err(Luks2Error::KeyVerificationFailed);
        }
        log::info!("luks2: master key verified");
    }

    Ok(master_key)
}
