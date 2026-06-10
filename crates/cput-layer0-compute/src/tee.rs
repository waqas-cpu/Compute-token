//! TEE attestation verification (production path).
//!
//! Production nodes must present a vendor-signed enclave quote. The reference
//! verifier accepts caller-supplied measurements for pipeline demos only.

use cput_core::{CputError, CputResult};

/// Verified enclave identity after quote validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeeIdentity {
    /// MRENCLAVE / measurement digest.
    pub measurement: [u8; 32],
    /// TCB security version (vendor-specific).
    pub tcb_version: u32,
}

/// Validates hardware TEE quotes before node registration (R0.1 production).
pub trait TeeVerifier: Send + Sync {
    /// Verify a raw attestation quote and extract the enclave measurement.
    fn verify_quote(&self, quote: &[u8]) -> CputResult<TeeIdentity>;
}

/// Prototype verifier — **not for mainnet**. Accepts any 32-byte measurement.
pub struct ReferenceTeeVerifier;

impl TeeVerifier for ReferenceTeeVerifier {
    fn verify_quote(&self, quote: &[u8]) -> CputResult<TeeIdentity> {
        if quote.len() < 32 {
            return Err(CputError::RuleViolation {
                rule: "R0.1",
                detail: format!("TEE quote too short: {} bytes", quote.len()),
            });
        }
        let mut measurement = [0u8; 32];
        measurement.copy_from_slice(&quote[..32]);
        Ok(TeeIdentity {
            measurement,
            tcb_version: 0,
        })
    }
}
