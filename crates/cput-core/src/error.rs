//! Unified error type for the CPUT protocol.
//!
//! Every layer and gate returns [`CputError`] so that failures can be
//! propagated across the vertical stack and surfaced at integration gates
//! with a uniform shape.

use thiserror::Error;

/// Result alias used throughout the workspace.
pub type CputResult<T> = Result<T, CputError>;

/// Canonical error taxonomy for the protocol.
#[derive(Debug, Error)]
pub enum CputError {
    /// A post-quantum signature failed verification.
    #[error("pqc signature verification failed: {0}")]
    SignatureInvalid(String),

    /// A KEM (key encapsulation) operation failed.
    #[error("pqc kem failure: {0}")]
    KemFailure(String),

    /// Key generation / lifecycle failure.
    #[error("pqc key error: {0}")]
    KeyError(String),

    /// A zero-knowledge proof failed verification.
    #[error("zk proof verification failed: {0}")]
    ProofInvalid(String),

    /// A threshold signature did not reach the required quorum.
    #[error("threshold not met: have {have}, need {need}")]
    ThresholdNotMet {
        /// Number of valid signatures collected.
        have: usize,
        /// Number of signatures required by the threshold rule.
        need: usize,
    },

    /// Multi-agent consensus could not be reached within tolerance.
    #[error(
        "agent consensus failed: spread {spread_bps} bps exceeds tolerance {tolerance_bps} bps"
    )]
    ConsensusFailed {
        /// Observed spread between agent proposals, in basis points.
        spread_bps: u32,
        /// Maximum permitted spread, in basis points.
        tolerance_bps: u32,
    },

    /// A value violated a documented protocol invariant / rule.
    #[error("rule violation [{rule}]: {detail}")]
    RuleViolation {
        /// Stable identifier of the violated rule.
        rule: &'static str,
        /// Human-readable detail about the violation.
        detail: String,
    },

    /// An integration gate rejected a crossing payload.
    #[error("gate `{gate}` rejected payload: {reason}")]
    GateRejected {
        /// Identifier of the integration gate that rejected the payload.
        gate: &'static str,
        /// Reason the gate rejected the payload.
        reason: String,
    },

    /// Replay / nonce / sequence violation.
    #[error("replay protection failure: {0}")]
    Replay(String),

    /// The anti-inflation circuit breaker is engaged.
    #[error("circuit breaker engaged: {0}")]
    CircuitBreaker(String),

    /// Serialization / deserialization failure.
    #[error("codec error: {0}")]
    Codec(String),

    /// A compliance / transfer-restriction rule blocked the action.
    #[error("compliance blocked: {0}")]
    ComplianceBlocked(String),
}
