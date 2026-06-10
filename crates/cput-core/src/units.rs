//! Economic and physical units.

use crate::error::{CputError, CputResult};
use serde::{Deserialize, Serialize};

/// Verified compute volume, expressed in GFLOPs delivered within an epoch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Gflops(pub u128);

impl Gflops {
    /// Saturating addition (compute volumes are aggregated across nodes).
    #[must_use]
    pub fn saturating_add(self, other: Gflops) -> Gflops {
        Gflops(self.0.saturating_add(other.0))
    }

    /// Convert to Sui `u64` on-chain representation (bounded).
    pub fn to_chain_u64(self) -> CputResult<u64> {
        u64::try_from(self.0).map_err(|_| {
            CputError::Chain(format!("GFLOPs {} exceeds Sui u64::MAX", self.0))
        })
    }
}

/// $CPUT amount in base units (smallest indivisible unit, 8 decimals on-chain).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TokenAmount(pub u128);

impl TokenAmount {
    /// Saturating addition.
    #[must_use]
    pub fn saturating_add(self, other: TokenAmount) -> TokenAmount {
        TokenAmount(self.0.saturating_add(other.0))
    }

    /// Apply a basis-points share, rounding down. e.g. `apply_bps(7000)` == 70%.
    #[must_use]
    pub fn apply_bps(self, bps: Bps) -> TokenAmount {
        TokenAmount(self.0.saturating_mul(u128::from(bps.0)) / 10_000)
    }

    /// Convert to Sui `u64` base units (bounded).
    pub fn to_chain_u64(self) -> CputResult<u64> {
        u64::try_from(self.0).map_err(|_| {
            CputError::Chain(format!("token amount {} exceeds Sui u64::MAX", self.0))
        })
    }
}

/// Basis points: 1 bps = 0.01%. `10_000` bps == 100%.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Bps(pub u16);

impl Bps {
    /// 100%.
    pub const FULL: Bps = Bps(10_000);

    /// Returns true if the value is a valid percentage (<= 100%).
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.0 <= 10_000
    }
}

/// A quality-weighted node score in basis points (latency percentile, uptime).
/// `10_000` == perfect quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QualityScore(pub u16);

impl QualityScore {
    /// Clamp to the valid `0..=10_000` range.
    #[must_use]
    pub fn clamped(self) -> QualityScore {
        QualityScore(self.0.min(10_000))
    }
}
