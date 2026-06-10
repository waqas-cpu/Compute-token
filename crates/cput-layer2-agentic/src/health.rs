//! Agent quorum health assessment (R2.2 operational surface).
//!
//! Operators use this to verify all [`MintingAgent`] instances are present,
//! within consensus tolerance, and bound to the committed emission policy before
//! mint instructions are relayed to Sui.

use crate::{EmissionPolicy, MintingAgent};
use cput_core::ids::AgentId;
use cput_core::units::Gflops;
use cput_core::{policy, CputResult};
use serde::{Deserialize, Serialize};

/// Per-agent health snapshot for monitoring and readiness probes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentHealth {
    /// Agent index in the quorum.
    pub id: AgentId,
    /// Agent kind label (e.g. `formula-agent`).
    pub kind: String,
    /// Hex-encoded ML-DSA public key.
    pub pubkey_hex: String,
    /// Independent proposal for the epoch (base units).
    pub proposal: u128,
    /// Deviation from base emission, in basis points.
    pub bias_bps: i32,
    /// Whether this agent is within the quorum tolerance band.
    pub within_tolerance: bool,
}

/// Aggregate quorum health used by `cput-agent --health` and relayer preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuorumHealth {
    /// Number of agents in the quorum.
    pub quorum_size: usize,
    /// Required quorum size per protocol policy.
    pub required_quorum: usize,
    /// Spread between min and max proposals, in basis points.
    pub spread_bps: u32,
    /// Maximum permitted spread.
    pub tolerance_bps: u32,
    /// Consensus total (mean of proposals) when healthy.
    pub consensus_total: u128,
    /// Hex-encoded committed emission policy hash.
    pub policy_hash_hex: String,
    /// Per-agent breakdown.
    pub agents: Vec<AgentHealth>,
    /// Overall readiness: quorum met and spread within tolerance.
    pub healthy: bool,
}

/// Assess quorum health without producing a mint instruction.
pub fn assess_quorum_health(
    agents: &[MintingAgent],
    policy: &EmissionPolicy,
    verified: Gflops,
) -> CputResult<QuorumHealth> {
    let required = policy::AGENT_QUORUM_SIZE;
    let tolerance = u32::from(policy::CONSENSUS_TOLERANCE_BPS.0);
    let policy_hash_hex = hex::encode(policy.policy_hash());

    if agents.len() < required {
        return Ok(QuorumHealth {
            quorum_size: agents.len(),
            required_quorum: required,
            spread_bps: u32::MAX,
            tolerance_bps: tolerance,
            consensus_total: 0,
            policy_hash_hex,
            agents: Vec::new(),
            healthy: false,
        });
    }

    let proposals: Vec<u128> = agents
        .iter()
        .map(|a| a.propose(policy, verified))
        .collect();
    let min = *proposals.iter().min().expect("non-empty quorum");
    let max = *proposals.iter().max().expect("non-empty quorum");
    let spread_bps = if min == 0 {
        0
    } else {
        ((max - min).saturating_mul(10_000) / min) as u32
    };
    let within_band = spread_bps <= tolerance;
    let consensus_total = if min == 0 {
        0
    } else {
        proposals.iter().sum::<u128>() / proposals.len() as u128
    };

    let agent_rows: Vec<AgentHealth> = agents
        .iter()
        .zip(proposals.iter())
        .map(|(agent, proposal)| AgentHealth {
            id: agent.id,
            kind: agent.kind_label().into(),
            pubkey_hex: hex::encode(agent.public_key()),
            proposal: *proposal,
            bias_bps: agent.bias_bps,
            within_tolerance: within_band,
        })
        .collect();

    Ok(QuorumHealth {
        quorum_size: agents.len(),
        required_quorum: required,
        spread_bps,
        tolerance_bps: tolerance,
        consensus_total,
        policy_hash_hex,
        agents: agent_rows,
        healthy: within_band && agents.len() >= required,
    })
}
