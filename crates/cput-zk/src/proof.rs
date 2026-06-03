//! Proof container and the backend trait.

use cput_core::CputResult;
use serde::{Deserialize, Serialize};

/// Identifier of the proving system that produced a proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofSystemId {
    /// UltraPlonk (Noir / Barretenberg) — the production system.
    UltraPlonk,
    /// Deterministic reference backend (development / integration only).
    Reference,
}

/// A self-describing proof object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proof {
    /// Proving system used.
    pub system: ProofSystemId,
    /// Commitment to the public inputs (32-byte digest).
    pub public_inputs_commitment: [u8; 32],
    /// Opaque proof bytes (verifier-defined encoding).
    pub bytes: Vec<u8>,
    /// Number of statements folded in (1 for a single proof; N for aggregates).
    pub folded: u32,
}

impl Proof {
    /// Hex of the public-inputs commitment — the value emitted on-chain as
    /// `zk_proof_hash`.
    #[must_use]
    pub fn commitment_hex(&self) -> String {
        hex::encode(self.public_inputs_commitment)
    }
}

/// A pluggable proving backend. Protocol code depends only on this trait, so
/// the reference backend can be swapped for Barretenberg/Noir without changes
/// upstream.
pub trait ProofBackend {
    /// Prove a statement that serializes to `public_inputs`.
    fn prove(&self, public_inputs: &[u8]) -> CputResult<Proof>;

    /// Verify a proof against the given `public_inputs`.
    fn verify(&self, proof: &Proof, public_inputs: &[u8]) -> CputResult<()>;

    /// Recursively aggregate a batch of proofs into a single proof, amortising
    /// on-chain verification cost (~95% per the architecture).
    fn aggregate(&self, proofs: &[Proof]) -> CputResult<Proof>;

    /// Verify an aggregate proof against the ordered batch of public inputs it
    /// claims to cover.
    fn verify_aggregate(&self, aggregate: &Proof, batch_inputs: &[Vec<u8>]) -> CputResult<()>;
}
