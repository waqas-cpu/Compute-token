//! # cput-audit — Immutable audit registry (HORIZONTAL)
//!
//! Implements the append-only, SLH-DSA-signed Merkle audit log described in
//! Layer 5: *"Every mint, burn, slash, governance action, and bridge transfer
//! is committed to an append-only Merkle tree … the root is signed with
//! SLH-DSA at each epoch."*
//!
//! It is horizontal because every layer writes audit entries through it.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod merkle;

use cput_core::ids::EpochId;
use cput_core::CputResult;
use cput_pqc::envelope::SealedEnvelope;
use cput_pqc::slhdsa::SlhDsaKeypair;
use cput_pqc::AlgorithmRegistry;
use merkle::{build_proof, compute_root, hash_leaf, MerkleProof};
use serde::{Deserialize, Serialize};

/// The kind of protocol action being recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditAction {
    /// Tokens minted for an epoch.
    Mint {
        /// Total amount minted (base units).
        total: u128,
    },
    /// Tokens burned via the demand-side market.
    Burn {
        /// Amount burned (base units).
        amount: u128,
    },
    /// A staker was slashed.
    Slash {
        /// Stake slashed (base units).
        amount: u128,
    },
    /// A governance action executed.
    Governance {
        /// Proposal identifier.
        proposal_id: u64,
    },
    /// A cross-chain bridge transfer occurred.
    Bridge {
        /// Destination chain identifier.
        dest_chain: u32,
        /// Amount bridged (base units).
        amount: u128,
    },
}

/// A single append-only audit entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Epoch the action belongs to.
    pub epoch: EpochId,
    /// Monotonic sequence number within the log.
    pub sequence: u64,
    /// The action recorded.
    pub action: AuditAction,
    /// Hash of the originating payload (e.g. mint instruction hash).
    pub payload_hash: [u8; 32],
}

impl AuditEntry {
    /// Canonical bytes used as the Merkle leaf preimage.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("AuditEntry serialization is infallible")
    }

    fn leaf(&self) -> [u8; 32] {
        hash_leaf(&self.canonical_bytes())
    }
}

/// An append-only audit log with a Merkle accumulator.
#[derive(Debug, Clone, Default)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
    leaves: Vec<[u8; 32]>,
}

/// An SLH-DSA-signed commitment to the audit-log root at an epoch boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedRoot {
    /// Epoch at which the root was committed.
    pub epoch: EpochId,
    /// Number of entries covered.
    pub size: u64,
    /// The Merkle root.
    pub root: [u8; 32],
    /// SLH-DSA signature over `(epoch, size, root)`.
    pub envelope: SealedEnvelope,
}

impl AuditLog {
    /// Create an empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an action and return the assigned sequence number.
    pub fn append(&mut self, epoch: EpochId, action: AuditAction, payload_hash: [u8; 32]) -> u64 {
        let sequence = self.entries.len() as u64;
        let entry = AuditEntry {
            epoch,
            sequence,
            action,
            payload_hash,
        };
        self.leaves.push(entry.leaf());
        self.entries.push(entry);
        sequence
    }

    /// Number of entries in the log.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if the log has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Current Merkle root over all entries.
    #[must_use]
    pub fn root(&self) -> [u8; 32] {
        compute_root(&self.leaves)
    }

    /// Build an inclusion proof for the entry at `sequence`.
    #[must_use]
    pub fn inclusion_proof(&self, sequence: u64) -> Option<MerkleProof> {
        build_proof(&self.leaves, sequence as usize)
    }

    /// Sign the current root with the DAO governance SLH-DSA key.
    pub fn sign_root(&self, epoch: EpochId, key: &SlhDsaKeypair) -> CputResult<SignedRoot> {
        let root = self.root();
        let size = self.entries.len() as u64;
        let mut msg = Vec::with_capacity(48);
        msg.extend_from_slice(&epoch.0.to_le_bytes());
        msg.extend_from_slice(&size.to_le_bytes());
        msg.extend_from_slice(&root);
        let envelope = SealedEnvelope::seal_slhdsa(key, msg)?;
        Ok(SignedRoot {
            epoch,
            size,
            root,
            envelope,
        })
    }
}

impl SignedRoot {
    /// Verify the SLH-DSA signature over the committed root.
    pub fn verify(&self, registry: &AlgorithmRegistry) -> CputResult<()> {
        self.envelope.verify(registry)
    }
}
