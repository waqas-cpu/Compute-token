//! Distributed key generation and threshold signatures (production path).
//!
//! The prototype counts independent ML-DSA signatures toward a threshold.
//! Production replaces this with a real DKG ceremony and threshold scheme.

use cput_core::{CputError, CputResult};

/// Threshold signature scheme used by the oracle DON.
pub trait ThresholdScheme: Send + Sync {
    /// Number of operators in the set.
    fn set_size(&self) -> usize;
    /// Signatures required to accept a report.
    fn threshold(&self) -> usize;
    /// Whether `share_count` valid shares meet the threshold.
    fn quorum_met(&self, share_count: usize) -> bool;
}

/// Prototype scheme: independent ML-DSA sigs counted to t-of-n (not true threshold crypto).
pub struct IndependentSigScheme {
    set_size: usize,
    threshold: usize,
}

impl IndependentSigScheme {
    /// Build the scheme from protocol policy constants.
    #[must_use]
    pub fn from_policy(set_size: usize, threshold: usize) -> Self {
        Self { set_size, threshold }
    }
}

impl ThresholdScheme for IndependentSigScheme {
    fn set_size(&self) -> usize {
        self.set_size
    }

    fn threshold(&self) -> usize {
        self.threshold
    }

    fn quorum_met(&self, share_count: usize) -> bool {
        share_count >= self.threshold
    }
}

/// DKG rotation gate — returns an error until a real ceremony is implemented.
pub fn assert_dkg_current(epoch: u64, last_rotation_epoch: u64, rotation_period: u64) -> CputResult<()> {
    if epoch.saturating_sub(last_rotation_epoch) > rotation_period {
        return Err(CputError::RuleViolation {
            rule: "DKG_ROTATION",
            detail: format!(
                "oracle DKG shares stale: last rotation epoch {last_rotation_epoch}, current {epoch}"
            ),
        });
    }
    Ok(())
}
