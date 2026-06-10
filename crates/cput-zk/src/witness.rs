//! Private witnesses for the compute and reasoning circuits.
//!
//! Witnesses are never relayed on-chain — only their commitments appear inside
//! proofs. Production Noir circuits consume the same logical fields.

use cput_core::ids::WorkloadHash;
use cput_core::units::Gflops;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Private inputs to the per-node compute circuit (Layer 0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeWitness {
    /// Measured GFLOPs from the execution trace (must equal public `gflops`).
    pub measured_gflops: u128,
    /// Merkle root of the private execution trace (binds to `workload_hash`).
    pub execution_trace_root: [u8; 32],
    /// TEE-measured output digest (binds to public `verified_hash`).
    pub tee_output_digest: [u8; 32],
}

impl ComputeWitness {
    /// Build a witness from attestation material (simulated or TEE-reported).
    #[must_use]
    pub fn from_attestation(
        gflops: Gflops,
        workload_hash: WorkloadHash,
        verified_hash: [u8; 32],
    ) -> Self {
        Self {
            measured_gflops: gflops.0,
            execution_trace_root: workload_trace_root(&workload_hash),
            tee_output_digest: verified_hash,
        }
    }

    /// Canonical SHA-256 commitment to the witness (appears inside proofs).
    #[must_use]
    pub fn commitment(&self) -> [u8; 32] {
        let bytes =
            serde_json::to_vec(self).expect("ComputeWitness serialization is infallible");
        let mut h = Sha256::new();
        h.update(b"cput.zk.witness.compute.v1");
        h.update(bytes);
        h.finalize().into()
    }
}

/// Private inputs to the agent-reasoning circuit (Layer 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasoningWitness {
    /// Hash of the intermediate formula evaluation trace.
    pub formula_trace_root: [u8; 32],
    /// Locally computed mint total (must equal public `mint_amount`).
    pub computed_mint: u128,
}

impl ReasoningWitness {
    /// Build a witness for the committed emission formula output.
    #[must_use]
    pub fn from_formula(epoch: u64, verified_gflops: u128, tokens_per_gflop: u128, mint: u128) -> Self {
        let mut h = Sha256::new();
        h.update(b"cput.zk.witness.reasoning.v1");
        h.update(epoch.to_le_bytes());
        h.update(verified_gflops.to_le_bytes());
        h.update(tokens_per_gflop.to_le_bytes());
        h.update(mint.to_le_bytes());
        Self {
            formula_trace_root: h.finalize().into(),
            computed_mint: mint,
        }
    }

    /// Canonical SHA-256 commitment to the witness.
    #[must_use]
    pub fn commitment(&self) -> [u8; 32] {
        let bytes =
            serde_json::to_vec(self).expect("ReasoningWitness serialization is infallible");
        let mut h = Sha256::new();
        h.update(b"cput.zk.witness.reasoning.body.v1");
        h.update(bytes);
        h.finalize().into()
    }
}

fn workload_trace_root(workload_hash: &WorkloadHash) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"cput.zk.trace.root.v1");
    h.update(workload_hash.0);
    h.finalize().into()
}
