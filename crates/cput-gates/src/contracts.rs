//! Typed boundary contracts — the only payloads permitted to cross layers.
//!
//! Each contract corresponds to an arrow in the architecture's vertical stack.
//! A contract pairs a signed body with whatever cryptographic evidence the
//! receiving gate must check (ZK proofs, threshold signatures).

use cput_core::ids::{AgentId, EpochId, NodeId, OracleId, WorkloadHash};
use cput_core::units::{Gflops, QualityScore, TokenAmount};
use cput_pqc::envelope::{SealedEnvelope, Signed};
use cput_pqc::AlgorithmId;
use cput_zk::Proof;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Identifies a layer in the vertical stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layer {
    /// L0 — physical compute infrastructure.
    Compute,
    /// L1 — oracle & attestation network.
    Oracle,
    /// L2 — agentic minting layer.
    Agentic,
    /// L4 — tokenomics engine.
    Tokenomics,
    /// L5 — settlement & governance.
    Settlement,
}

// ---------------------------------------------------------------------------
// Gate 0→1: signed compute attestation packet.
// ---------------------------------------------------------------------------

/// Body of a per-node compute attestation (Layer 0 output).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationBody {
    /// Reporting node's permanent identity.
    pub node_id: NodeId,
    /// Epoch the compute was delivered in.
    pub epoch: EpochId,
    /// Verified GFLOPs delivered.
    pub gflops: Gflops,
    /// Hash of the executed workload.
    pub workload_hash: WorkloadHash,
    /// Thermal envelope in degrees Celsius (telemetry sanity bound).
    pub thermal_envelope_c: u16,
    /// TEE-reported verified output hash.
    pub verified_hash: [u8; 32],
    /// Enclave measurement (MRENCLAVE) the workload ran under.
    pub enclave_measurement: [u8; 32],
}

impl AttestationBody {
    /// Derive the node identity that a given ML-DSA public key commits to.
    #[must_use]
    pub fn node_id_for_key(public_key: &[u8]) -> NodeId {
        let digest: [u8; 32] = Sha256::digest(public_key).into();
        NodeId(digest)
    }
}

/// A Layer 0 → Layer 1 attestation packet: ML-DSA-signed body plus its ZK
/// compute proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationPacket {
    /// ML-DSA-87 signed attestation body.
    pub signed: Signed<AttestationBody>,
    /// Per-node UltraPlonk compute proof.
    pub compute_proof: Proof,
}

// ---------------------------------------------------------------------------
// Gate 1→2: threshold-signed epoch report.
// ---------------------------------------------------------------------------

/// One node's contribution within an epoch report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeScore {
    /// Node identity.
    pub node_id: NodeId,
    /// Verified GFLOPs attributed to the node.
    pub gflops: Gflops,
    /// Quality weight (latency percentile, uptime ratio).
    pub quality: QualityScore,
}

/// Body of the canonical epoch report (Layer 1 output).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochReportBody {
    /// Epoch being reported.
    pub epoch: EpochId,
    /// Sum of verified GFLOPs across all contributing nodes.
    pub verified_gflops_total: Gflops,
    /// Per-node scores.
    pub node_scores: Vec<NodeScore>,
    /// Commitment of the aggregated ZK proof (`zk_proof_hash`).
    pub zk_proof_commitment: [u8; 32],
    /// Epoch at which the dispute window closes.
    pub dispute_window_close: EpochId,
}

/// A t-of-n threshold signature: independent oracle signatures over the same
/// epoch-report body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdSignature {
    /// (oracle id, signature envelope) pairs.
    pub shares: Vec<(OracleId, SealedEnvelope)>,
}

/// A Layer 1 → Layer 2 epoch report: body, recursive aggregate proof, and the
/// oracle threshold signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochReport {
    /// Canonical report body.
    pub body: EpochReportBody,
    /// Recursive UltraPlonk aggregate proof covering all node proofs.
    pub aggregated_proof: Proof,
    /// Ordered leaf public inputs used to re-verify the aggregate at Gate 1→2.
    pub leaf_public_inputs: Vec<Vec<u8>>,
    /// 5-of-9 ML-DSA-65 threshold signature over the body.
    pub threshold_sig: ThresholdSignature,
}

// ---------------------------------------------------------------------------
// Gate 2→4: agent-signed mint instruction.
// ---------------------------------------------------------------------------

/// Body of one agent's independent mint proposal (R2.2 evidence).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentProposalBody {
    /// Agent index in the quorum.
    pub agent_id: AgentId,
    /// Epoch being proposed for.
    pub epoch: EpochId,
    /// This agent's proposed mint total (base units).
    pub proposal: u128,
    /// Committed emission policy hash the agent executed.
    pub policy_hash: [u8; 32],
}

/// Per-node token allocation within a mint instruction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeAllocation {
    /// Node receiving the allocation.
    pub node_id: NodeId,
    /// Amount allocated (base units).
    pub amount: TokenAmount,
}

/// Body of a mint instruction (Layer 2 output).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MintInstructionBody {
    /// Epoch the mint applies to.
    pub epoch: EpochId,
    /// Total tokens to mint this epoch.
    pub total: TokenAmount,
    /// Per-node allocations (sum must equal `total`).
    pub allocations: Vec<NodeAllocation>,
    /// Hash of the committed agent reasoning trace.
    pub reasoning_trace_hash: [u8; 32],
    /// Hash binding the policy/system prompt the agents executed.
    pub policy_hash: [u8; 32],
}

/// A Layer 2 → Layer 4 mint instruction: ML-DSA-signed body plus ZK proof of
/// correct formula execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintInstruction {
    /// ML-DSA-87 signed mint-instruction body (coordinator seal).
    pub signed: Signed<MintInstructionBody>,
    /// Per-agent ML-DSA-87 signed proposals (R2.2 quorum evidence).
    pub agent_proposals: Vec<Signed<AgentProposalBody>>,
    /// ZK proof of correct agent reasoning.
    pub reasoning_proof: Proof,
}

// ---------------------------------------------------------------------------
// Gate 4→5: settlement receipt.
// ---------------------------------------------------------------------------

/// Body of a settlement receipt (Layer 4 output).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementReceiptBody {
    /// Settled epoch.
    pub epoch: EpochId,
    /// Total minted this epoch.
    pub minted_total: TokenAmount,
    /// Amount routed to compute providers.
    pub providers: TokenAmount,
    /// Amount routed to oracle stakers.
    pub oracle: TokenAmount,
    /// Amount routed to the DAO treasury.
    pub treasury: TokenAmount,
    /// Amount routed to the burn reserve.
    pub burn_reserve: TokenAmount,
    /// Audit-log root after this epoch's entries.
    pub audit_root: [u8; 32],
}

/// A Layer 4 → Layer 5 settlement receipt (ML-DSA-signed by the engine).
pub type SettlementReceipt = Signed<SettlementReceiptBody>;

// ---------------------------------------------------------------------------
// Downward gate 5→2/4: governance policy update.
// ---------------------------------------------------------------------------

/// Body of a governance policy update (Layer 5 → minting parameters).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyUpdateBody {
    /// Epoch from which the update takes effect.
    pub effective_epoch: EpochId,
    /// New per-epoch mint ceiling.
    pub mint_ceiling: TokenAmount,
    /// Whether the anti-inflation circuit breaker is engaged.
    pub circuit_breaker_engaged: bool,
    /// Approved algorithm set (crypto-agility).
    pub approved_algorithms: Vec<AlgorithmId>,
}

/// A Layer 5 → Layer 2/4 policy update (SLH-DSA-signed governance action).
pub type PolicyUpdate = Signed<PolicyUpdateBody>;
