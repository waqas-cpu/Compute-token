//! # cput-gates — Integration gates
//!
//! This crate is the explicit home of the system's **integration gates**: the
//! typed contracts that may cross a layer boundary ([`contracts`]) and the
//! validation rules each boundary enforces ([`gate`]).
//!
//! ## Why a dedicated crate?
//!
//! In the horizontal/vertical decomposition (see `ARCHITECTURE.md`), gates are
//! the *seams* between vertical layers. Centralising the boundary contracts
//! here means:
//! - a layer cannot fabricate or bypass a neighbour's contract — it must
//!   produce/consume the shared, signed type;
//! - every cross-layer rule (signature, crypto-agility, ZK proof, threshold
//!   quorum, economic invariant) lives in exactly one auditable place;
//! - the gate functions never trust upstream data — they re-verify it.
//!
//! ## The gates
//!
//! | Gate | Boundary | Carries |
//! |------|----------|---------|
//! | [`gate::admit_attestation`] | L0 → L1 | [`contracts::AttestationPacket`] |
//! | [`gate::admit_epoch_report`] | L1 → L2 | [`contracts::EpochReport`] |
//! | [`gate::admit_mint_instruction`] | L2 → L4 | [`contracts::MintInstruction`] |
//! | [`gate::admit_settlement_receipt`] | L4 → L5 | [`contracts::SettlementReceipt`] |
//! | [`gate::admit_policy_update`] | L5 → L2/L4 (downward) | [`contracts::PolicyUpdate`] |

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod contracts;
pub mod gate;

pub use contracts::{
    AttestationBody, AttestationPacket, EpochReport, EpochReportBody, Layer, MintInstruction,
    MintInstructionBody, NodeAllocation, NodeScore, PolicyUpdate, PolicyUpdateBody,
    SettlementReceipt, SettlementReceiptBody, ThresholdSignature,
};
pub use gate::{
    admit_attestation, admit_epoch_report, admit_mint_instruction, admit_policy_update,
    admit_settlement_receipt, GateConfig,
};
