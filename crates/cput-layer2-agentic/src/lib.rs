//! # cput-layer2-agentic — VERTICAL LAYER 2
//!
//! The autonomous minting layer. A quorum of independent agents each consume
//! the verified [`EpochReport`] (admitting it through **Gate 1→2**),
//! independently apply the committed emission formula, and must agree within a
//! tight tolerance before a single coordinating agent emits the ML-DSA-signed,
//! ZK-proven [`MintInstruction`] consumed by Layer 4 (**Gate 2→4** producer
//! side).
//!
//! ## Layer rules (invariants this layer guarantees)
//!
//! - **R2.1 Verified input only.** The epoch report is re-admitted at Gate 1→2
//!   before any agent reasons over it.
//! - **R2.2 Independent quorum.** [`AGENT_QUORUM_SIZE`](cput_core::policy::AGENT_QUORUM_SIZE)
//!   agents each produce a proposal; the spread must stay within
//!   [`CONSENSUS_TOLERANCE_BPS`](cput_core::policy::CONSENSUS_TOLERANCE_BPS).
//! - **R2.3 Conservation.** Per-node allocations sum to the consensus total.
//! - **R2.4 Committed policy.** The emission policy is hashed and bound into the
//!   instruction and the reasoning proof, so Gate 2→4 can check the agents ran
//!   the approved formula.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use cput_core::ids::OracleId;
use cput_core::ids::{AgentId, EpochId};
use cput_core::units::{Gflops, TokenAmount};
use cput_core::{policy, CputError, CputResult};
use cput_gates::contracts::{
    EpochReport, MintInstruction, MintInstructionBody, NodeAllocation, NodeScore,
};
use cput_gates::gate::{admit_epoch_report, GateConfig};
use cput_pqc::envelope::Signed;
use cput_pqc::mldsa::MlDsaKeypair;
use cput_zk::{encode_statement, AgentReasoningStatement, ProofBackend};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The committed emission policy the agents execute. Hashing this yields the
/// `policy_hash` bound into every mint instruction (R2.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmissionPolicy {
    /// Tokens (base units) minted per verified GFLOP.
    pub tokens_per_gflop: u128,
    /// Human-readable policy version tag.
    pub version: String,
}

impl EmissionPolicy {
    /// The canonical hash bound into mint instructions and reasoning proofs.
    #[must_use]
    pub fn policy_hash(&self) -> [u8; 32] {
        let bytes = serde_json::to_vec(self).expect("EmissionPolicy serialization is infallible");
        Sha256::digest(bytes).into()
    }

    /// Base emission for a verified compute total (pre-consensus).
    #[must_use]
    pub fn base_emission(&self, verified: Gflops) -> u128 {
        verified.0.saturating_mul(self.tokens_per_gflop)
    }
}

/// One agent instance in the minting quorum. `bias_bps` models the small,
/// bounded variation between independent model runs.
pub struct MintingAgent {
    /// Index within the quorum.
    pub id: AgentId,
    /// Deterministic per-agent deviation from the base emission, in bps.
    pub bias_bps: i32,
}

impl MintingAgent {
    /// Construct an agent with no deviation from the committed formula.
    #[must_use]
    pub fn new(id: AgentId) -> Self {
        Self { id, bias_bps: 0 }
    }

    /// This agent's independent proposal for the epoch's mint total.
    #[must_use]
    pub fn propose(&self, policy: &EmissionPolicy, verified: Gflops) -> u128 {
        let base = policy.base_emission(verified);
        let signed = i128::from(self.bias_bps);
        let adjusted = (base as i128) * (10_000 + signed) / 10_000;
        adjusted.max(0) as u128
    }
}

/// The minting quorum: independent agents plus one coordinating ML-DSA key that
/// signs the consensus instruction.
pub struct AgentQuorum<'a, B: ProofBackend> {
    cfg: GateConfig<'a, B>,
    agents: Vec<MintingAgent>,
    coordinator: MlDsaKeypair,
    policy: EmissionPolicy,
}

impl<'a, B: ProofBackend> AgentQuorum<'a, B> {
    /// Assemble a quorum over the committed emission policy.
    pub fn new(
        cfg: GateConfig<'a, B>,
        agents: Vec<MintingAgent>,
        coordinator: MlDsaKeypair,
        policy: EmissionPolicy,
    ) -> Self {
        Self {
            cfg,
            agents,
            coordinator,
            policy,
        }
    }

    /// The coordinating agent's registered public key (checked at Gate 2→4).
    #[must_use]
    pub fn coordinator_key(&self) -> Vec<u8> {
        self.coordinator.public_bytes()
    }

    /// Compute the consensus mint total or fail if the quorum is too small or
    /// the proposals spread beyond tolerance (R2.2).
    fn consensus_total(&self, verified: Gflops) -> CputResult<u128> {
        if self.agents.len() < policy::AGENT_QUORUM_SIZE {
            return Err(CputError::ThresholdNotMet {
                have: self.agents.len(),
                need: policy::AGENT_QUORUM_SIZE,
            });
        }
        let proposals: Vec<u128> = self
            .agents
            .iter()
            .map(|a| a.propose(&self.policy, verified))
            .collect();
        let min = *proposals.iter().min().expect("non-empty quorum");
        let max = *proposals.iter().max().expect("non-empty quorum");
        if min == 0 {
            // Degenerate epoch (no verified compute) — nothing to mint.
            return Ok(0);
        }
        let spread_bps = ((max - min).saturating_mul(10_000) / min) as u32;
        if spread_bps > u32::from(policy::CONSENSUS_TOLERANCE_BPS.0) {
            return Err(CputError::ConsensusFailed {
                spread_bps,
                tolerance_bps: u32::from(policy::CONSENSUS_TOLERANCE_BPS.0),
            });
        }
        let sum: u128 = proposals.iter().sum();
        Ok(sum / proposals.len() as u128)
    }

    /// Allocate the consensus total across nodes proportional to verified
    /// GFLOPs, folding any rounding remainder into the first node (R2.3).
    fn allocate(scores: &[NodeScore], total: u128, verified_total: Gflops) -> Vec<NodeAllocation> {
        if verified_total.0 == 0 || total == 0 {
            return scores
                .iter()
                .map(|s| NodeAllocation {
                    node_id: s.node_id,
                    amount: TokenAmount(0),
                })
                .collect();
        }
        let mut allocations: Vec<NodeAllocation> = Vec::with_capacity(scores.len());
        let mut distributed: u128 = 0;
        for s in scores {
            let share = total.saturating_mul(s.gflops.0) / verified_total.0;
            distributed = distributed.saturating_add(share);
            allocations.push(NodeAllocation {
                node_id: s.node_id,
                amount: TokenAmount(share),
            });
        }
        // Fold rounding remainder into the first allocation (R2.3 exact sum).
        if let Some(first) = allocations.first_mut() {
            let remainder = total.saturating_sub(distributed);
            first.amount = TokenAmount(first.amount.0.saturating_add(remainder));
        }
        allocations
    }

    /// Run the full minting cycle for a verified epoch report (producer side of
    /// Gate 2→4). Enforces R2.1–R2.4.
    pub fn mint_for_report(
        &self,
        oracle_keys: &BTreeMap<OracleId, Vec<u8>>,
        report: &EpochReport,
    ) -> CputResult<MintInstruction> {
        // R2.1 — only reason over a report that clears Gate 1→2.
        admit_epoch_report(&self.cfg, oracle_keys, report)?;

        let verified = report.body.verified_gflops_total;
        let total = self.consensus_total(verified)?; // R2.2
        let allocations = Self::allocate(&report.body.node_scores, total, verified); // R2.3

        let policy_hash = self.policy.policy_hash();
        let reasoning_trace_hash: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(b"cput.l2.reasoning.v1");
            h.update(report.body.epoch.0.to_le_bytes());
            h.update(total.to_le_bytes());
            h.update(policy_hash);
            h.finalize().into()
        };

        let body = MintInstructionBody {
            epoch: report.body.epoch,
            total: TokenAmount(total),
            allocations,
            reasoning_trace_hash,
            policy_hash,
        };

        // R2.4 — ZK proof that the committed policy produced `total` from the
        // verified GFLOPs.
        let statement = AgentReasoningStatement {
            epoch: report.body.epoch,
            verified_gflops: verified,
            mint_amount: total,
            policy_hash,
        };
        let reasoning_proof = self.cfg.backend.prove(&encode_statement(&statement)?)?;

        let signed = Signed::seal_mldsa(&self.coordinator, body)?;
        Ok(MintInstruction {
            signed,
            reasoning_proof,
        })
    }

    /// The next epoch after `epoch` (cycle bookkeeping helper).
    #[must_use]
    pub fn next_epoch(epoch: EpochId) -> EpochId {
        epoch.next()
    }
}
