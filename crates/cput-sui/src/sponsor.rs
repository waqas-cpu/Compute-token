//! Gas sponsorship configuration for relayer-submitted transactions.
//!
//! When enabled, the relayer passes `--gas-owner` so a sponsor wallet pays gas
//! while the relayer address remains the transaction sender (account-abstraction
//! prep for zkLogin / passkey signers later).

use serde::{Deserialize, Serialize};

/// Optional gas sponsorship (gas station / DAO pays relayer gas).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SponsorConfig {
    /// Enable sponsored gas for relayer transactions.
    #[serde(default)]
    pub enabled: bool,
    /// Address of the gas sponsor wallet (`--gas-owner`).
    pub gas_owner: Option<String>,
    /// Per-transaction gas budget (MIST).
    #[serde(default = "default_gas_budget")]
    pub gas_budget: u64,
    /// Shared `GasSponsorState` object ID (for `record_sponsored_epoch`).
    #[serde(default)]
    pub gas_sponsor_state: Option<String>,
}

fn default_gas_budget() -> u64 {
    100_000_000
}

impl Default for SponsorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            gas_owner: None,
            gas_budget: default_gas_budget(),
            gas_sponsor_state: None,
        }
    }
}

impl SponsorConfig {
    /// True when relayer should attach a gas owner to each CLI call.
    #[must_use]
    pub fn active(&self) -> bool {
        self.enabled && self.gas_owner.as_ref().is_some_and(|g| !g.is_empty() && !g.contains("REPLACE"))
    }
}
