//! Strongly-typed identifiers used across the protocol.
//!
//! Newtypes prevent accidental mixing of, e.g., an [`EpochId`] with a raw
//! `u64`, which is a common source of cross-layer integration bugs.

use serde::{Deserialize, Serialize};

/// Monotonic epoch counter. One epoch == one minting cycle (target 6 min).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EpochId(pub u64);

impl EpochId {
    /// The epoch immediately following this one.
    #[must_use]
    pub fn next(self) -> Self {
        EpochId(self.0 + 1)
    }
}

impl core::fmt::Display for EpochId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "epoch#{}", self.0)
    }
}

/// A compute node's permanent identity: the 32-byte digest of its committed
/// ML-DSA-87 public key (Layer 0 registration).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 32]);

impl NodeId {
    /// Hex rendering of the identity digest.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl core::fmt::Display for NodeId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "node:{}", &self.to_hex()[..16])
    }
}

/// Index of an oracle operator within the decentralised oracle network (DON).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OracleId(pub u16);

/// Index of an agent instance within the multi-agent minting quorum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AgentId(pub u8);

/// Opaque workload / job hash reported by Layer 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkloadHash(pub [u8; 32]);

impl WorkloadHash {
    /// Hex rendering.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}
