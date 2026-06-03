//! Public statements proven by the ZK circuits.
//!
//! A statement captures only the *public inputs* of a circuit — the private
//! witness (workload internals, customer data, model weights) never appears.

use cput_core::ids::{EpochId, NodeId, WorkloadHash};
use cput_core::units::Gflops;
use serde::{Deserialize, Serialize};

/// Public inputs to the per-node compute proof circuit (Layer 0 → Layer 1).
///
/// Proves: *"I executed workload `workload_hash` of `gflops` magnitude inside
/// enclave `enclave_measurement` during `epoch`"* — revealing only these
/// aggregates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeStatement {
    /// Node that produced the proof.
    pub node_id: NodeId,
    /// Epoch the compute was delivered in.
    pub epoch: EpochId,
    /// Aggregate verified GFLOPs (the only quantitative reveal).
    pub gflops: Gflops,
    /// Hash of the executed workload.
    pub workload_hash: WorkloadHash,
    /// Enclave measurement (MRENCLAVE) the work ran under.
    pub enclave_measurement: [u8; 32],
}

/// Public inputs to the agent-reasoning proof circuit (Layer 2 → Layer 4).
///
/// Proves the mint amount was computed by faithfully applying the policy
/// formula to the verified epoch report, without revealing model weights.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentReasoningStatement {
    /// Epoch the mint decision applies to.
    pub epoch: EpochId,
    /// Total verified GFLOPs the formula consumed as input.
    pub verified_gflops: Gflops,
    /// The mint amount the formula produced (public output).
    pub mint_amount: u128,
    /// Hash binding the committed policy/system-prompt the agent ran under.
    pub policy_hash: [u8; 32],
}
