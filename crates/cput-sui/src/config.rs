//! Sui deployment configuration loaded from TOML.

use crate::sponsor::SponsorConfig;
use cput_core::{CputError, CputResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Pool addresses configured on-chain via `cput::configure_pools`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolAddresses {
    /// Compute-provider pool.
    pub provider: String,
    /// Oracle-staker pool.
    pub oracle: String,
    /// DAO treasury pool.
    pub treasury: String,
    /// Burn-reserve pool.
    pub burn_reserve: String,
}

/// Published package and shared-object IDs for the CPUT Sui deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiDeployment {
    /// JSON-RPC endpoint (testnet / mainnet / localnet).
    pub rpc_url: String,
    /// Published package ID (`0x…`).
    pub package_id: String,
    /// Shared `ProtocolState` object ID.
    pub protocol_state: String,
    /// Shared `OracleState` object ID.
    pub oracle_state: String,
    /// Shared `MintLog` object ID.
    pub mint_log: String,
    /// Shared `GovernanceState` object ID.
    pub governance_state: String,
    /// Shared `StakeBook` object ID.
    pub stake_book: String,
    /// Shared `AuditRegistry` object ID (optional).
    #[serde(default)]
    pub audit_registry: Option<String>,
    /// Shared `PqcAnchorBook` object ID (optional).
    #[serde(default)]
    pub pqc_anchor_book: Option<String>,
    /// Shared `AgentRegistry` object ID (AI agent identity).
    #[serde(default)]
    pub agent_registry: Option<String>,
    /// Shared `AgentQuorumBook` object ID (agent-centric mint approvals).
    #[serde(default)]
    pub agent_quorum_book: Option<String>,
    /// Owned `AdminCap` object ID held by the relayer.
    pub admin_cap: String,
    /// Distribution pool addresses.
    pub pools: PoolAddresses,
    /// Path to `sui` CLI binary (for transaction submission).
    #[serde(default = "default_sui_bin")]
    pub sui_cli: String,
    /// When true, build intents and log CLI commands without submitting.
    #[serde(default)]
    pub dry_run: bool,
    /// Optional gas sponsorship (relayer sender, sponsor pays gas).
    #[serde(default)]
    pub sponsor: SponsorConfig,
}

fn default_sui_bin() -> String {
    "sui".into()
}

impl SuiDeployment {
    /// Load deployment config from a TOML file.
    pub fn from_file(path: impl AsRef<Path>) -> CputResult<Self> {
        let text = std::fs::read_to_string(path.as_ref())
            .map_err(|e| CputError::Chain(format!("read config: {e}")))?;
        toml::from_str(&text).map_err(|e| CputError::Chain(format!("parse config: {e}")))
    }
}
