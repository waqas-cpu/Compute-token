//! Chain-facing transaction intents and event mirrors.

use serde::{Deserialize, Serialize};

/// Arguments for `oracle_verifier::submit_report`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmitReportIntent {
    /// Epoch number.
    pub epoch: u64,
    /// Verified GFLOP total.
    pub verified_gflops: u64,
    /// ZK aggregate commitment bytes.
    pub zk_commitment: Vec<u8>,
    /// Distinct signing oracle indices (0..ORACLE_SET_SIZE-1).
    pub signer_indices: Vec<u16>,
}

/// Arguments for `agent_quorum::approve_epoch_mint`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentQuorumIntent {
    /// Epoch number.
    pub epoch: u64,
    /// Distinct agent indices (0..AGENT_QUORUM_SIZE-1).
    pub agent_ids: Vec<u8>,
    /// Per-agent proposed mint totals (base units).
    pub proposals: Vec<u64>,
    /// Coordinator consensus total (must match integer average).
    pub total: u64,
    /// Committed emission policy hash (32 bytes).
    pub policy_hash: [u8; 32],
    /// Agent reasoning trace hash (32 bytes).
    pub reasoning_trace_hash: [u8; 32],
}

/// Arguments for `minting::execute_mint`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecuteMintIntent {
    /// Epoch number.
    pub epoch: u64,
    /// Total mint amount (base units).
    pub total: u64,
}

/// Arguments for `cput::set_ceiling`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetCeilingIntent {
    /// New per-epoch mint ceiling.
    pub mint_ceiling: u64,
    /// Circuit breaker engaged.
    pub circuit_breaker_engaged: bool,
}

/// Arguments for `burn::pay_access_fee`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PayAccessFeeIntent {
    /// Fee amount in CPUT base units.
    pub fee: u64,
    /// Coin object ID to spend (relayer-held or user-provided).
    pub coin_object_id: String,
}

/// On-chain `EpochMinted` event mirror (from `minting` module).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnChainEpochMinted {
    /// Epoch number.
    pub epoch: u64,
    /// Total minted.
    pub total: u64,
    /// Provider share.
    pub providers: u64,
    /// Oracle share.
    pub oracle: u64,
    /// Treasury share.
    pub treasury: u64,
    /// Burn-reserve share.
    pub burn_reserve: u64,
    /// Verified GFLOPs from oracle report.
    pub verified_gflops: u64,
}

/// On-chain `FeeRouted` event mirror (from `burn` module).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnChainFeeRouted {
    /// Payer address.
    pub payer: String,
    /// Total fee.
    pub fee: u64,
    /// Burned portion.
    pub burned: u64,
    /// Provider portion.
    pub providers: u64,
    /// Treasury portion.
    pub treasury: u64,
}
