//! # cput-core
//!
//! Foundational, dependency-light domain model shared by every CPUT crate.
//!
//! This crate sits at the bottom of the dependency graph. It deliberately
//! contains **no** layer logic, no cryptography, and no I/O — only the
//! vocabulary (identifiers, units), the unified [`error::CputError`], and the
//! quantitative [`policy`] rules that the rest of the system enforces.
//!
//! ## Decomposition role
//!
//! In the horizontal/vertical decomposition (see `ARCHITECTURE.md`),
//! `cput-core` is the *shared kernel*: it is the only crate that every
//! horizontal fabric and every vertical layer is allowed to depend on.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod ids;
pub mod policy;
pub mod units;

pub use error::{CputError, CputResult};
pub use ids::{AgentId, EpochId, NodeId, OracleId, WorkloadHash};
pub use units::{Bps, Gflops, QualityScore, TokenAmount};
