//! Protocol policy constants.
//!
//! These values encode the quantitative *rules* stated in the architecture
//! reference. They are referenced by the vertical layers, the integration
//! gates, and the on-chain governor so that a single source of truth governs
//! the off-chain and (mirrored) on-chain behaviour.

use crate::units::Bps;

/// Target epoch length in seconds (6 minutes).
pub const EPOCH_SECONDS: u64 = 360;

/// Dispute window after epoch settlement, in epochs (~42 minutes).
pub const DISPUTE_WINDOW_EPOCHS: u64 = 7;

/// Oracle DON: total operators.
pub const ORACLE_SET_SIZE: usize = 9;

/// Oracle DON: signatures required to accept an epoch report (t-of-n).
pub const ORACLE_THRESHOLD: usize = 5;

/// Fraction of reported compute events challenged each epoch (5%).
pub const CHALLENGE_SAMPLE_BPS: Bps = Bps(500);

/// DKG key-share rotation cadence (every 1,000 epochs).
pub const DKG_ROTATION_EPOCHS: u64 = 1_000;

/// Number of independent agent instances in the minting quorum.
pub const AGENT_QUORUM_SIZE: usize = 3;

/// Maximum spread between agent mint proposals for consensus (±2%).
pub const CONSENSUS_TOLERANCE_BPS: Bps = Bps(200);

// --- Distribution split (must sum to 100%). ---
/// Compute providers' share of newly minted tokens.
pub const SPLIT_PROVIDERS_BPS: Bps = Bps(7_000);
/// Oracle stakers' share.
pub const SPLIT_ORACLE_BPS: Bps = Bps(1_500);
/// DAO treasury share.
pub const SPLIT_TREASURY_BPS: Bps = Bps(1_000);
/// Burn-reserve share.
pub const SPLIT_BURN_RESERVE_BPS: Bps = Bps(500);

// --- Demand-side burn market (must sum to 100%). ---
/// Portion of access fees burned (reduces supply).
pub const BURN_FEE_BURNED_BPS: Bps = Bps(2_000);
/// Portion of access fees routed to providers.
pub const BURN_FEE_PROVIDER_BPS: Bps = Bps(7_500);
/// Portion of access fees routed to treasury.
pub const BURN_FEE_TREASURY_BPS: Bps = Bps(500);

// --- Elastic supply governor. ---
/// Utilisation below this lowers the ceiling (oversupply risk).
pub const UTIL_LOW_BPS: Bps = Bps(6_000);
/// Utilisation above this may raise the ceiling (scarcity risk).
pub const UTIL_HIGH_BPS: Bps = Bps(9_000);
/// Ceiling decrease when utilisation is low (5%).
pub const CEILING_DECREASE_BPS: Bps = Bps(500);
/// Maximum ceiling increase when utilisation is high (3%).
pub const CEILING_INCREASE_BPS: Bps = Bps(300);
/// TWAP window for utilisation smoothing, in epochs (7 days @ 6-min epochs).
pub const TWAP_WINDOW_EPOCHS: u64 = 1_680;

// --- Staking & slashing. ---
/// Withdrawal delay for staked collateral, in epochs.
pub const WITHDRAWAL_DELAY_EPOCHS: u64 = 14;
/// Fraction of a fraudulent node's stake that is burned.
pub const SLASH_BURN_BPS: Bps = Bps(5_000);
/// Fraction of a fraudulent node's stake awarded to the fraud-proof submitter.
pub const SLASH_BOUNTY_BPS: Bps = Bps(5_000);

/// Emergency council multisig: signatures required out of total.
pub const COUNCIL_THRESHOLD: usize = 7;
/// Emergency council multisig: total members.
pub const COUNCIL_SET_SIZE: usize = 11;

/// Returns true iff the four distribution-split shares sum to exactly 100%.
#[must_use]
pub fn distribution_split_is_valid() -> bool {
    SPLIT_PROVIDERS_BPS.0 + SPLIT_ORACLE_BPS.0 + SPLIT_TREASURY_BPS.0 + SPLIT_BURN_RESERVE_BPS.0
        == Bps::FULL.0
}

/// Returns true iff the three burn-market shares sum to exactly 100%.
#[must_use]
pub fn burn_market_split_is_valid() -> bool {
    BURN_FEE_BURNED_BPS.0 + BURN_FEE_PROVIDER_BPS.0 + BURN_FEE_TREASURY_BPS.0 == Bps::FULL.0
}
