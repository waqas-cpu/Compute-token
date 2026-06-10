//! # cput-pqc — Post-Quantum Cryptography fabric (HORIZONTAL)
//!
//! Layer 3 of the architecture is described as *"not a discrete service
//! component — a pervasive security fabric"*. We model it exactly that way: as
//! a **horizontal** crate that every layer and gate depends on, rather than a
//! vertical layer slice.
//!
//! All three NIST PQC standards are deployed in their documented roles:
//! - [`mldsa`] — ML-DSA-87 (FIPS 204): attestations, mint signing, call auth.
//! - [`mlkem`] — ML-KEM-1024 (FIPS 203): encrypted node↔oracle/agent channels.
//! - [`slhdsa`] — SLH-DSA-256s (FIPS 205): long-lived roots, audit-log roots.
//!
//! [`registry`] provides crypto-agility (the approved-algorithm registry) and
//! [`envelope`] provides the algorithm-tagged [`envelope::SealedEnvelope`] that
//! crosses every integration gate.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod anchor;
pub mod envelope;
pub mod mldsa;
pub mod mlkem;
pub mod registry;
pub mod slhdsa;

pub use anchor::{batch_digests, envelope_digest, pubkey_digest};
pub use envelope::{SealedEnvelope, Signed};
pub use registry::{AlgorithmId, AlgorithmRegistry};
