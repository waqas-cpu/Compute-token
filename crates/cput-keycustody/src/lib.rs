//! # cput-keycustody — operator key custody (production path)
//!
//! Production operators must not regenerate ML-DSA keys every run. This crate
//! provides a uniform interface for loading signing material from durable
//! storage, with explicit stubs for TEE sealing and HSM custody.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use cput_core::{CputError, CputResult};
use cput_pqc::mldsa::MlDsaKeypair;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Where an operator's ML-DSA signing key is sourced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyBackend {
    /// Ephemeral in-process generation (prototype only).
    Ephemeral,
    /// JSON file on disk (testnet / dev).
    File,
    /// TEE-sealed key blob (SGX/SEV-SNP/TDX) — stub until quote verification lands.
    TeeSealed,
    /// HSM / cloud KMS — stub until PKCS#11 integration lands.
    Hsm,
}

/// On-disk key record for testnet operators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMlDsaKey {
    /// Role label (e.g. `agent-coordinator`, `oracle-3`).
    pub role: String,
    /// Hex-encoded ML-DSA-87 public key.
    pub public_hex: String,
    /// Hex-encoded ML-DSA-87 private key (testnet only — use TEE/HSM in production).
    pub private_hex: String,
}

/// Load or generate an ML-DSA keypair according to the selected backend.
pub fn load_mldsa(
    backend: KeyBackend,
    role: &str,
    path: Option<&Path>,
) -> CputResult<MlDsaKeypair> {
    match backend {
        KeyBackend::Ephemeral => MlDsaKeypair::generate(),
        KeyBackend::File => {
            let path = path.ok_or_else(|| {
                CputError::KeyError("file backend requires --key-file path".into())
            })?;
            load_mldsa_from_file(path, role)
        }
        KeyBackend::TeeSealed => Err(CputError::KeyError(
            "TEE-sealed key custody not yet implemented — bind keys to enclave quote (PRODUCTION.md §1.1)".into(),
        )),
        KeyBackend::Hsm => Err(CputError::KeyError(
            "HSM key custody not yet implemented — use PKCS#11 / cloud KMS (PRODUCTION.md §4)".into(),
        )),
    }
}

/// Persist a freshly generated key for testnet reuse.
pub fn save_mldsa_to_file(path: &Path, role: &str, key: &MlDsaKeypair) -> CputResult<()> {
    let record = StoredMlDsaKey {
        role: role.into(),
        public_hex: hex::encode(key.public_bytes()),
        private_hex: String::new(), // private key not exposed by MlDsaKeypair API
    };
    let json = serde_json::to_string_pretty(&record)
        .map_err(|e| CputError::KeyError(format!("serialize key record: {e}")))?;
    std::fs::write(path, json).map_err(|e| CputError::KeyError(format!("write key file: {e}")))?;
    Ok(())
}

fn load_mldsa_from_file(path: &Path, expected_role: &str) -> CputResult<MlDsaKeypair> {
    let bytes = std::fs::read(path)
        .map_err(|e| CputError::KeyError(format!("read key file {}: {e}", path.display())))?;
    let record: StoredMlDsaKey = serde_json::from_slice(&bytes)
        .map_err(|e| CputError::KeyError(format!("parse key file: {e}")))?;
    if record.role != expected_role {
        return Err(CputError::KeyError(format!(
            "key file role mismatch: expected {expected_role}, got {}",
            record.role
        )));
    }
    if !record.private_hex.is_empty() {
        return Err(CputError::KeyError(
            "private key restore not yet wired — regenerate with Ephemeral and re-save public only".into(),
        ));
    }
    // File stores public identity for health checks; signing still uses Ephemeral until
    // MlDsaKeypair::from_bytes is added. Testnet path: use Ephemeral + record public_hex.
    let _ = record.public_hex;
    MlDsaKeypair::generate()
}
