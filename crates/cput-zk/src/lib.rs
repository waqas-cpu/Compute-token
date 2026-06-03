//! # cput-zk — Zero-Knowledge proof fabric (HORIZONTAL)
//!
//! Abstraction over the UltraPlonk proving system (Noir circuits, Barretenberg
//! backend) used by the architecture for:
//! - per-node **compute proofs** (Layer 0/1): *"I executed workload W of X
//!   GFLOP magnitude … revealing only the aggregate GFLOP count"*,
//! - **recursive aggregation** of N node proofs into one (Layer 1),
//! - **agent-reasoning proofs** (Layer 2): proof of correct mint-formula
//!   execution without revealing model weights.
//!
//! ## Backend strategy
//!
//! The trait [`ProofBackend`] decouples protocol logic from the prover. This
//! crate ships a [`reference::ReferenceBackend`] — a deterministic,
//! hash-commitment backend that is **sound for wiring/integration but is NOT
//! zero-knowledge or cryptographically binding**. The production backend
//! (`barretenberg`/Noir) plugs in behind the same trait. The integration point
//! is documented in `PRODUCTION.md`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod proof;
pub mod reference;
pub mod statements;

pub use proof::{Proof, ProofBackend, ProofSystemId};
pub use reference::ReferenceBackend;
pub use statements::{AgentReasoningStatement, ComputeStatement};

use cput_core::{CputError, CputResult};
use serde::Serialize;

/// Canonically encode a statement's public inputs. Provers and integration
/// gates MUST both route public inputs through this function so the bytes a
/// proof is generated over exactly match the bytes it is verified against.
pub fn encode_statement<T: Serialize>(statement: &T) -> CputResult<Vec<u8>> {
    serde_json::to_vec(statement).map_err(|e| CputError::Codec(format!("statement encode: {e}")))
}
