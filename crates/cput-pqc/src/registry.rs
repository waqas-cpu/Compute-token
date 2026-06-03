//! Crypto-agility: algorithm identifiers and the approved-algorithm registry.
//!
//! Every signature and ciphertext in the protocol carries an [`AlgorithmId`]
//! header. The [`AlgorithmRegistry`] mirrors the on-chain registry of approved
//! algorithms, allowing new NIST standards to be added — and old ones
//! deprecated with a migration window — via governance, without redeploying
//! verifier logic.

use cput_core::{CputError, CputResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A cryptographic algorithm deployed in the CPUT PQC fabric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AlgorithmId {
    /// ML-DSA-87 signatures (FIPS 204).
    MlDsa87,
    /// ML-KEM-1024 key encapsulation (FIPS 203).
    MlKem1024,
    /// SLH-DSA-SHA2-256s signatures (FIPS 205).
    SlhDsaSha2_256s,
}

impl AlgorithmId {
    /// One-byte wire tag included in signature / ciphertext headers.
    #[must_use]
    pub fn tag(self) -> u8 {
        match self {
            AlgorithmId::MlDsa87 => 0x04,
            AlgorithmId::MlKem1024 => 0x03,
            AlgorithmId::SlhDsaSha2_256s => 0x05,
        }
    }

    /// Decode an algorithm from its wire tag.
    pub fn from_tag(tag: u8) -> CputResult<Self> {
        match tag {
            0x04 => Ok(AlgorithmId::MlDsa87),
            0x03 => Ok(AlgorithmId::MlKem1024),
            0x05 => Ok(AlgorithmId::SlhDsaSha2_256s),
            other => Err(CputError::KeyError(format!(
                "unknown algorithm tag {other:#x}"
            ))),
        }
    }

    /// True if this algorithm produces signatures (vs. key encapsulation).
    #[must_use]
    pub fn is_signature(self) -> bool {
        matches!(self, AlgorithmId::MlDsa87 | AlgorithmId::SlhDsaSha2_256s)
    }
}

/// The set of currently approved algorithms (crypto-agility registry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmRegistry {
    approved: BTreeSet<AlgorithmId>,
}

impl Default for AlgorithmRegistry {
    /// The genesis registry approves all three deployed NIST standards.
    fn default() -> Self {
        let mut approved = BTreeSet::new();
        approved.insert(AlgorithmId::MlDsa87);
        approved.insert(AlgorithmId::MlKem1024);
        approved.insert(AlgorithmId::SlhDsaSha2_256s);
        Self { approved }
    }
}

impl AlgorithmRegistry {
    /// True if `algo` is currently approved for use.
    #[must_use]
    pub fn is_approved(&self, algo: AlgorithmId) -> bool {
        self.approved.contains(&algo)
    }

    /// Approve a new algorithm (governance action).
    pub fn approve(&mut self, algo: AlgorithmId) {
        self.approved.insert(algo);
    }

    /// Deprecate an algorithm (governance action). Returns the rule error if it
    /// was not approved to begin with.
    pub fn deprecate(&mut self, algo: AlgorithmId) -> CputResult<()> {
        if self.approved.remove(&algo) {
            Ok(())
        } else {
            Err(CputError::RuleViolation {
                rule: "PQC.CRYPTOAGILITY.DEPRECATE",
                detail: format!("{algo:?} is not currently approved"),
            })
        }
    }

    /// Reject use of an unapproved algorithm at a verification boundary.
    pub fn require_approved(&self, algo: AlgorithmId) -> CputResult<()> {
        if self.is_approved(algo) {
            Ok(())
        } else {
            Err(CputError::RuleViolation {
                rule: "PQC.CRYPTOAGILITY.APPROVED",
                detail: format!("{algo:?} is not in the approved-algorithm registry"),
            })
        }
    }
}
