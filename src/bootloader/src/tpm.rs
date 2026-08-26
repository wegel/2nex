//! TPM2 key unsealing for LUKS2 master key
//!
//! retrieves the LUKS2 master key from TPM NV storage, sealed to PCR policy.
//! uses raw TPM2 commands via UEFI TCG2 protocol.

use alloc::vec;
use alloc::vec::Vec;
use uefi::proto::tcg::v2::Tcg;

// TPM2 NV index for LUKS master key (in owner-defined space)
// different from conductor's 0x01800010 to avoid conflicts
// provisioning: tpm2_nvdefine 0x01800020 -s 64 -L policy.bin
const NV_INDEX_LUKS_MASTER_KEY: u32 = 0x01800020;

// master key size for AES-256-XTS (two 256-bit keys)
const MASTER_KEY_SIZE: u16 = 64;

// PCR indices for policy binding
const PCR_BOOTLOADER: u32 = 4; // UEFI measures loaded images to PCR 4
const PCR_SECUREBOOT: u32 = 7; // SecureBoot state

// TPM2 command codes
const TPM2_CC_START_AUTH_SESSION: u32 = 0x00000176;
const TPM2_CC_POLICY_PCR: u32 = 0x0000017F;
const TPM2_CC_NV_READ: u32 = 0x0000014E;
const TPM2_CC_FLUSH_CONTEXT: u32 = 0x00000165;

// TPM2 tags
const TPM2_ST_NO_SESSIONS: u16 = 0x8001;
const TPM2_ST_SESSIONS: u16 = 0x8002;

// TPM2 handles
const TPM2_RH_NULL: u32 = 0x40000007;
#[allow(dead_code)]
const TPM2_RS_PW: u32 = 0x40000009; // password session, for future use

// TPM2 algorithm IDs
const TPM2_ALG_SHA256: u16 = 0x000B;
const TPM2_ALG_NULL: u16 = 0x0010;

// TPM2 session types
const TPM2_SE_POLICY: u8 = 0x01;

// TPM2 response codes
const TPM2_RC_SUCCESS: u32 = 0x00000000;

#[derive(Debug)]
#[allow(dead_code)]
pub enum TpmError {
    ProtocolNotFound,
    TpmNotPresent,
    CommandFailed(u32),
    InvalidResponse,
    NvNotFound,
}

/// key material that can be returned from TPM
/// designed to support future Option 2 (passphrase material)
#[allow(dead_code)]
pub enum TpmKeyMaterial {
    /// direct master key (Option 1) - skip KDF
    MasterKey(Vec<u8>),
    /// passphrase material (Option 2, future) - run through KDF
    Passphrase(Vec<u8>),
}

/// try to unseal LUKS master key from TPM
///
/// returns None if TPM is not available or unsealing fails
/// (caller should fall back to passphrase input)
pub fn try_unseal_master_key() -> Option<Vec<u8>> {
    match unseal_from_tpm() {
        Ok(TpmKeyMaterial::MasterKey(key)) => {
            log::info!("tpm: successfully unsealed master key");
            Some(key)
        }
        Ok(TpmKeyMaterial::Passphrase(_)) => {
            // future: return passphrase material for KDF
            log::warn!("tpm: passphrase material not yet supported");
            None
        }
        Err(e) => {
            log::info!("tpm: unsealing failed: {:?}", e);
            None
        }
    }
}

fn unseal_from_tpm() -> Result<TpmKeyMaterial, TpmError> {
    // open TCG2 protocol
    let tcg_handle =
        uefi::boot::get_handle_for_protocol::<Tcg>().map_err(|_| TpmError::ProtocolNotFound)?;

    let mut tcg = uefi::boot::open_protocol_exclusive::<Tcg>(tcg_handle)
        .map_err(|_| TpmError::ProtocolNotFound)?;

    // check TPM is present
    let capability = tcg.get_capability().map_err(|_| TpmError::TpmNotPresent)?;
    if !capability.tpm_present() {
        return Err(TpmError::TpmNotPresent);
    }

    log::info!("tpm: TPM2 present, attempting to unseal key...");

    // start policy session
    let session_handle = start_policy_session(&mut tcg)?;
    log::debug!("tpm: started policy session: 0x{:08x}", session_handle);

    // apply PCR policy (try PCR 7 first, then PCR 4)
    // we use PolicyOR in provisioning, so either PCR can satisfy
    let pcr_result = apply_pcr_policy(&mut tcg, session_handle, PCR_SECUREBOOT);
    if pcr_result.is_err() {
        log::debug!("tpm: PCR 7 policy failed, trying PCR 4");
        apply_pcr_policy(&mut tcg, session_handle, PCR_BOOTLOADER)?;
    }

    // read from NV using the policy session
    let key_data = nv_read_with_session(
        &mut tcg,
        session_handle,
        NV_INDEX_LUKS_MASTER_KEY,
        MASTER_KEY_SIZE,
    )?;

    // flush the session
    let _ = flush_context(&mut tcg, session_handle);

    if key_data.len() != MASTER_KEY_SIZE as usize {
        return Err(TpmError::InvalidResponse);
    }

    Ok(TpmKeyMaterial::MasterKey(key_data))
}

/// start a policy session for NV access
fn start_policy_session(tcg: &mut Tcg) -> Result<u32, TpmError> {
    // TPM2_StartAuthSession command
    // structure: tag(2) + size(4) + cc(4) + tpmKey(4) + bind(4) + nonceCaller(2+32) +
    //            encryptedSalt(2) + sessionType(1) + symmetric(2+2+2) + authHash(2)
    let mut cmd = Vec::with_capacity(64);

    // header
    cmd.extend_from_slice(&TPM2_ST_NO_SESSIONS.to_be_bytes());
    cmd.extend_from_slice(&0u32.to_be_bytes()); // size placeholder
    cmd.extend_from_slice(&TPM2_CC_START_AUTH_SESSION.to_be_bytes());

    // tpmKey = TPM_RH_NULL (unbound session)
    cmd.extend_from_slice(&TPM2_RH_NULL.to_be_bytes());
    // bind = TPM_RH_NULL (unbound session)
    cmd.extend_from_slice(&TPM2_RH_NULL.to_be_bytes());

    // nonceCaller (TPM2B_NONCE) - 32 bytes of zeros is fine for our use
    cmd.extend_from_slice(&32u16.to_be_bytes()); // size
    cmd.extend_from_slice(&[0u8; 32]); // nonce

    // encryptedSalt (TPM2B_ENCRYPTED_SECRET) - empty for unbound session
    cmd.extend_from_slice(&0u16.to_be_bytes());

    // sessionType = TPM_SE_POLICY
    cmd.push(TPM2_SE_POLICY);

    // symmetric (TPMT_SYM_DEF) = NULL
    cmd.extend_from_slice(&TPM2_ALG_NULL.to_be_bytes());

    // authHash = SHA256
    cmd.extend_from_slice(&TPM2_ALG_SHA256.to_be_bytes());

    // fix up size
    let size = cmd.len() as u32;
    cmd[2..6].copy_from_slice(&size.to_be_bytes());

    // send command
    let mut response = vec![0u8; 256];
    tcg.submit_command(&cmd, &mut response)
        .map_err(|_| TpmError::CommandFailed(0))?;

    // parse response
    let rc = parse_response_code(&response)?;
    if rc != TPM2_RC_SUCCESS {
        return Err(TpmError::CommandFailed(rc));
    }

    // session handle is at offset 10 (after tag(2) + size(4) + rc(4))
    if response.len() < 14 {
        return Err(TpmError::InvalidResponse);
    }
    let session_handle =
        u32::from_be_bytes([response[10], response[11], response[12], response[13]]);

    Ok(session_handle)
}

/// apply PolicyPCR to a session
fn apply_pcr_policy(tcg: &mut Tcg, session_handle: u32, pcr_index: u32) -> Result<(), TpmError> {
    // TPM2_PolicyPCR command
    // structure: tag(2) + size(4) + cc(4) + policySession(4) +
    //            pcrDigest(2+0) + pcrs(4+2+1+3)
    let mut cmd = Vec::with_capacity(32);

    // header
    cmd.extend_from_slice(&TPM2_ST_NO_SESSIONS.to_be_bytes());
    cmd.extend_from_slice(&0u32.to_be_bytes()); // size placeholder
    cmd.extend_from_slice(&TPM2_CC_POLICY_PCR.to_be_bytes());

    // policySession handle
    cmd.extend_from_slice(&session_handle.to_be_bytes());

    // pcrDigest (TPM2B_DIGEST) - empty means use current PCR values
    cmd.extend_from_slice(&0u16.to_be_bytes());

    // pcrs (TPML_PCR_SELECTION)
    cmd.extend_from_slice(&1u32.to_be_bytes()); // count = 1 selection
    cmd.extend_from_slice(&TPM2_ALG_SHA256.to_be_bytes()); // hash = SHA256
    cmd.push(3); // sizeofSelect = 3 bytes (PCR 0-23)

    // PCR selection bitmap (3 bytes covering PCR 0-23)
    let mut pcr_select = [0u8; 3];
    pcr_select[(pcr_index / 8) as usize] = 1 << (pcr_index % 8);
    cmd.extend_from_slice(&pcr_select);

    // fix up size
    let size = cmd.len() as u32;
    cmd[2..6].copy_from_slice(&size.to_be_bytes());

    // send command
    let mut response = vec![0u8; 64];
    tcg.submit_command(&cmd, &mut response)
        .map_err(|_| TpmError::CommandFailed(0))?;

    // check response
    let rc = parse_response_code(&response)?;
    if rc != TPM2_RC_SUCCESS {
        return Err(TpmError::CommandFailed(rc));
    }

    Ok(())
}

/// read from NV index using a policy session
fn nv_read_with_session(
    tcg: &mut Tcg,
    session_handle: u32,
    nv_index: u32,
    size: u16,
) -> Result<Vec<u8>, TpmError> {
    let command = build_nv_read_command(session_handle, nv_index, size);
    let mut response = vec![0u8; 256];
    tcg.submit_command(&command, &mut response)
        .map_err(|_| TpmError::CommandFailed(0))?;
    parse_nv_read_response(&response)
}

fn build_nv_read_command(session_handle: u32, nv_index: u32, size: u16) -> Vec<u8> {
    let mut cmd = Vec::with_capacity(64);
    cmd.extend_from_slice(&TPM2_ST_SESSIONS.to_be_bytes());
    cmd.extend_from_slice(&0u32.to_be_bytes());
    cmd.extend_from_slice(&TPM2_CC_NV_READ.to_be_bytes());
    let nv_handle = 0x01000000 | nv_index;
    cmd.extend_from_slice(&nv_handle.to_be_bytes());
    cmd.extend_from_slice(&nv_handle.to_be_bytes());
    let auth_area_start = cmd.len();
    cmd.extend_from_slice(&0u32.to_be_bytes());
    cmd.extend_from_slice(&session_handle.to_be_bytes());
    cmd.extend_from_slice(&0u16.to_be_bytes());
    cmd.push(0x01);
    cmd.extend_from_slice(&0u16.to_be_bytes());
    let auth_area_size = (cmd.len() - auth_area_start - 4) as u32;
    cmd[auth_area_start..auth_area_start + 4].copy_from_slice(&auth_area_size.to_be_bytes());
    cmd.extend_from_slice(&size.to_be_bytes());
    cmd.extend_from_slice(&0u16.to_be_bytes());
    let cmd_size = cmd.len() as u32;
    cmd[2..6].copy_from_slice(&cmd_size.to_be_bytes());
    cmd
}

fn parse_nv_read_response(response: &[u8]) -> Result<Vec<u8>, TpmError> {
    let rc = parse_response_code(response)?;
    if rc != TPM2_RC_SUCCESS {
        return Err(TpmError::CommandFailed(rc));
    }
    if response.len() < 16 {
        return Err(TpmError::InvalidResponse);
    }
    let param_size = u32::from_be_bytes([response[10], response[11], response[12], response[13]]);
    if param_size < 2 {
        return Err(TpmError::InvalidResponse);
    }
    let data_size = u16::from_be_bytes([response[14], response[15]]);
    if response.len() < 16 + data_size as usize {
        return Err(TpmError::InvalidResponse);
    }
    Ok(response[16..16 + data_size as usize].to_vec())
}

/// flush a session context
fn flush_context(tcg: &mut Tcg, handle: u32) -> Result<(), TpmError> {
    let mut cmd = Vec::with_capacity(16);

    cmd.extend_from_slice(&TPM2_ST_NO_SESSIONS.to_be_bytes());
    cmd.extend_from_slice(&14u32.to_be_bytes()); // fixed size
    cmd.extend_from_slice(&TPM2_CC_FLUSH_CONTEXT.to_be_bytes());
    cmd.extend_from_slice(&handle.to_be_bytes());

    let mut response = vec![0u8; 32];
    let _ = tcg.submit_command(&cmd, &mut response);

    Ok(())
}

/// parse response code from TPM response
fn parse_response_code(response: &[u8]) -> Result<u32, TpmError> {
    if response.len() < 10 {
        return Err(TpmError::InvalidResponse);
    }

    // response format: tag(2) + size(4) + responseCode(4)
    let rc = u32::from_be_bytes([response[6], response[7], response[8], response[9]]);
    Ok(rc)
}
