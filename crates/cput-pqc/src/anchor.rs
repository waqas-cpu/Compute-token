//! Signature digests for on-chain anchoring.
//!
//! Sui Move cannot verify ML-DSA-87 natively. The relayer verifies envelopes
//! off-chain; this module produces deterministic digests the on-chain
//! [`pqc_anchor`] module records for audit and replay protection.

use crate::envelope::SealedEnvelope;
use cput_core::CputResult;
use sha2::{Digest, Sha256};

/// SHA-256 digest of a sealed envelope (algorithm tag + pubkey + payload + sig).
#[must_use]
pub fn envelope_digest(env: &SealedEnvelope) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([env.algorithm.tag()]);
    h.update(env.signer_public.as_slice());
    h.update(env.payload.as_slice());
    h.update(env.signature.as_slice());
    h.finalize().into()
}

/// SHA-256 digest of a signer's public key (on-chain identity shorthand).
#[must_use]
pub fn pubkey_digest(public_key: &[u8]) -> [u8; 32] {
    Sha256::digest(public_key).into()
}

/// Collect digests for a batch of envelopes (coordinator + agent proposals).
pub fn batch_digests(envelopes: &[&SealedEnvelope]) -> CputResult<Vec<[u8; 32]>> {
    Ok(envelopes.iter().map(|e| envelope_digest(e)).collect())
}
