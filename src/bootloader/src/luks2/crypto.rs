//! LUKS2 cryptographic operations
//!
//! implements Argon2id key derivation, AES-XTS decryption, and AF-splitter.

use aes::cipher::KeyInit;
use aes::Aes256;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use sha2::{Digest as Sha2Digest, Sha256};
use xts_mode::{get_tweak_default, Xts128};

use super::json::Kdf;

#[derive(Debug)]
pub enum CryptoError {
    KdfError(String),
    InvalidKeySize,
    UnsupportedHash(String),
    UnsupportedKdf(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::KdfError(msg) => write!(f, "KDF error: {}", msg),
            CryptoError::InvalidKeySize => write!(f, "invalid key size"),
            CryptoError::UnsupportedHash(h) => write!(f, "unsupported hash: {}", h),
            CryptoError::UnsupportedKdf(k) => write!(f, "unsupported KDF: {}", k),
        }
    }
}

/// derive key using the KDF parameters from a keyslot
pub fn derive_key(passphrase: &[u8], kdf: &Kdf, output_len: usize) -> Result<Vec<u8>, CryptoError> {
    match kdf.kdf_type.as_str() {
        "argon2id" | "argon2i" => derive_key_argon2(passphrase, kdf, output_len),
        "pbkdf2" => derive_key_pbkdf2(passphrase, kdf, output_len),
        other => Err(CryptoError::UnsupportedKdf(String::from(other))),
    }
}

/// derive key using Argon2id/Argon2i
fn derive_key_argon2(
    passphrase: &[u8],
    kdf: &Kdf,
    output_len: usize,
) -> Result<Vec<u8>, CryptoError> {
    use argon2::{Algorithm, Argon2, Params, Version};

    let algorithm = if kdf.kdf_type == "argon2id" {
        Algorithm::Argon2id
    } else {
        Algorithm::Argon2i
    };

    let time_cost = kdf.time.unwrap_or(4);
    let memory_cost = kdf.memory.unwrap_or(1048576); // default 1GB in KB
    let parallelism = kdf.cpus.unwrap_or(4);

    log::info!(
        "argon2: time={}, memory={}KB, parallelism={}",
        time_cost,
        memory_cost,
        parallelism
    );

    let params = Params::new(memory_cost, time_cost, parallelism, Some(output_len))
        .map_err(|e| CryptoError::KdfError(alloc::format!("params: {}", e)))?;

    let argon2 = Argon2::new(algorithm, Version::V0x13, params);

    let mut output = vec![0u8; output_len];
    argon2
        .hash_password_into(passphrase, &kdf.salt, &mut output)
        .map_err(|e| CryptoError::KdfError(alloc::format!("hash: {}", e)))?;

    Ok(output)
}

/// derive key using PBKDF2-SHA256
fn derive_key_pbkdf2(
    passphrase: &[u8],
    kdf: &Kdf,
    output_len: usize,
) -> Result<Vec<u8>, CryptoError> {
    let hash = kdf.hash.as_deref().unwrap_or("sha256");
    if hash != "sha256" {
        return Err(CryptoError::UnsupportedHash(String::from(hash)));
    }

    let iterations = kdf.iterations.unwrap_or(100000);

    let mut output = vec![0u8; output_len];
    pbkdf2::pbkdf2_hmac::<Sha256>(passphrase, &kdf.salt, iterations, &mut output);

    Ok(output)
}

/// AES-256-XTS cipher for sector decryption
pub struct AesXts {
    xts: Xts128<Aes256>,
    sector_size: usize,
}

impl AesXts {
    /// create from 512-bit key (two 256-bit keys for XTS)
    pub fn new(key: &[u8], sector_size: usize) -> Result<Self, CryptoError> {
        if key.len() != 64 {
            return Err(CryptoError::InvalidKeySize);
        }

        use aes::cipher::generic_array::GenericArray;

        // XTS uses two AES-256 keys
        let cipher1 = Aes256::new(GenericArray::from_slice(&key[0..32]));
        let cipher2 = Aes256::new(GenericArray::from_slice(&key[32..64]));

        Ok(Self {
            xts: Xts128::new(cipher1, cipher2),
            sector_size,
        })
    }

    /// decrypt a single sector in place
    #[allow(dead_code)]
    pub fn decrypt_sector(&self, sector_index: u64, data: &mut [u8]) {
        debug_assert!(data.len() >= self.sector_size);
        let sector = &mut data[..self.sector_size];
        self.xts
            .decrypt_sector(sector, get_tweak_default(sector_index as u128));
    }

    /// decrypt multiple consecutive sectors in place
    pub fn decrypt_sectors(&self, start_sector: u64, data: &mut [u8]) {
        let num_sectors = data.len() / self.sector_size;
        for i in 0..num_sectors {
            let offset = i * self.sector_size;
            let sector = &mut data[offset..offset + self.sector_size];
            self.xts
                .decrypt_sector(sector, get_tweak_default((start_sector + i as u64) as u128));
        }
    }

    #[allow(dead_code)]
    pub fn sector_size(&self) -> usize {
        self.sector_size
    }
}

/// LUKS1-style anti-forensic splitter merge operation
///
/// recovers the master key from the striped key material
pub fn af_merge(
    split_key: &[u8],
    key_size: usize,
    stripes: u32,
    hash: &str,
) -> Result<Vec<u8>, CryptoError> {
    if hash != "sha256" {
        return Err(CryptoError::UnsupportedHash(String::from(hash)));
    }

    let stripe_size = key_size;
    let mut result = vec![0u8; key_size];

    for i in 0..stripes {
        let stripe_offset = (i as usize) * stripe_size;
        let stripe = &split_key[stripe_offset..stripe_offset + stripe_size];

        // XOR with diffused previous result
        let diffused = af_diffuse(&result);
        for j in 0..key_size {
            result[j] = diffused[j] ^ stripe[j];
        }
    }

    Ok(result)
}

/// diffuse function for anti-forensic splitter (SHA-256 based)
fn af_diffuse(data: &[u8]) -> Vec<u8> {
    let mut output = vec![0u8; data.len()];
    let hash_size = 32; // SHA-256 output size

    // process in 32-byte blocks
    let num_blocks = data.len().div_ceil(hash_size);

    for block_idx in 0..num_blocks {
        let start = block_idx * hash_size;
        let end = core::cmp::min(start + hash_size, data.len());
        let block_len = end - start;

        let mut hasher = Sha256::new();
        hasher.update((block_idx as u32).to_be_bytes());
        hasher.update(&data[start..end]);
        let hash = hasher.finalize();

        output[start..end].copy_from_slice(&hash[..block_len]);
    }

    output
}

/// verify master key against a PBKDF2 digest
pub fn verify_master_key(
    master_key: &[u8],
    salt: &[u8],
    expected_digest: &[u8],
    iterations: u32,
    hash: &str,
) -> Result<bool, CryptoError> {
    if hash != "sha256" {
        return Err(CryptoError::UnsupportedHash(String::from(hash)));
    }

    let mut derived = vec![0u8; expected_digest.len()];
    pbkdf2::pbkdf2_hmac::<Sha256>(master_key, salt, iterations, &mut derived);

    // constant-time comparison
    Ok(constant_time_eq(&derived, expected_digest))
}

/// constant-time equality comparison
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}
