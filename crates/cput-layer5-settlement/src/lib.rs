//! # cput-layer5-settlement — VERTICAL LAYER 5
//!
//! Settlement and governance. It consumes the [`SettlementReceipt`] (admitting
//! it through **Gate 4→5**), commits the epoch's audit root under an SLH-DSA
//! signature, runs DAO governance (quadratic voting), and emits the SLH-DSA
//! signed [`PolicyUpdate`] that flows back down through the **downward gate
//! 5→2/4** to retune minting parameters.
//!
//! ## Layer rules (invariants this layer guarantees)
//!
//! - **R5.1 Settlement finality.** A receipt is only finalised once it clears
//!   Gate 4→5 (signature + conservation + policy split).
//! - **R5.2 Signed audit root.** Each settled epoch's audit root is committed
//!   under an SLH-DSA signature (long-lived root of trust).
//! - **R5.3 Quadratic voting.** Voting power is the integer square root of a
//!   voter's token weight, mitigating plutocracy.
//! - **R5.4 Safe policy.** A zero mint ceiling is only emitted with the circuit
//!   breaker engaged (re-checked at the downward gate).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use cput_core::ids::EpochId;
use cput_core::units::TokenAmount;
use cput_core::{CputError, CputResult};
use cput_gates::contracts::{PolicyUpdate, PolicyUpdateBody, SettlementReceipt};
use cput_gates::gate::admit_settlement_receipt;
use cput_pqc::envelope::{SealedEnvelope, Signed};
use cput_pqc::slhdsa::SlhDsaKeypair;
use cput_pqc::{AlgorithmId, AlgorithmRegistry};
use serde::{Deserialize, Serialize};

/// An SLH-DSA-signed commitment to a settled epoch's audit root (R5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochCommitment {
    /// Settled epoch.
    pub epoch: EpochId,
    /// Total minted that epoch (echoed for convenience).
    pub minted_total: TokenAmount,
    /// Audit-log Merkle root committed for the epoch.
    pub audit_root: [u8; 32],
    /// SLH-DSA signature over `(epoch, minted_total, audit_root)`.
    pub envelope: SealedEnvelope,
}

/// A governance ballot cast over a proposal.
#[derive(Debug, Clone)]
pub struct Ballot {
    /// Token weight backing the vote (base units).
    pub token_weight: u128,
    /// `true` for, `false` against.
    pub support: bool,
}

/// A governance proposal to retune minting parameters.
#[derive(Debug, Clone)]
pub struct Proposal {
    /// Proposal identifier.
    pub id: u64,
    /// Epoch from which the change takes effect if it passes.
    pub effective_epoch: EpochId,
    /// Proposed new per-epoch mint ceiling.
    pub mint_ceiling: TokenAmount,
    /// Whether the proposal engages the anti-inflation circuit breaker.
    pub circuit_breaker_engaged: bool,
    /// Approved-algorithm set to publish (crypto-agility).
    pub approved_algorithms: Vec<AlgorithmId>,
}

/// Integer square root (floor) — the quadratic-voting weighting (R5.3).
#[must_use]
pub fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Outcome of tallying a proposal's ballots under quadratic voting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    /// Quadratic voting power in favour.
    pub for_power: u128,
    /// Quadratic voting power against.
    pub against_power: u128,
}

impl Tally {
    /// Whether the proposal passes (strict majority of quadratic power).
    #[must_use]
    pub fn passed(&self) -> bool {
        self.for_power > self.against_power
    }
}

/// The settlement & governance layer.
pub struct SettlementLayer<'a> {
    registry: &'a AlgorithmRegistry,
    governance_key: SlhDsaKeypair,
}

impl<'a> SettlementLayer<'a> {
    /// Construct the layer with the DAO's long-lived SLH-DSA governance key.
    pub fn new(registry: &'a AlgorithmRegistry, governance_key: SlhDsaKeypair) -> Self {
        Self {
            registry,
            governance_key,
        }
    }

    /// The governance public key (checked by the downward gate 5→2/4).
    #[must_use]
    pub fn governance_key(&self) -> Vec<u8> {
        self.governance_key.public_bytes()
    }

    /// Finalise an epoch: admit the receipt at Gate 4→5 and commit its audit
    /// root under an SLH-DSA signature (R5.1, R5.2).
    pub fn finalize_settlement(&self, receipt: &SettlementReceipt) -> CputResult<EpochCommitment> {
        admit_settlement_receipt(self.registry, receipt)?;
        let b = &receipt.body;
        let mut msg = Vec::with_capacity(48);
        msg.extend_from_slice(&b.epoch.0.to_le_bytes());
        msg.extend_from_slice(&b.minted_total.0.to_le_bytes());
        msg.extend_from_slice(&b.audit_root);
        let envelope = SealedEnvelope::seal_slhdsa(&self.governance_key, msg)?;
        Ok(EpochCommitment {
            epoch: b.epoch,
            minted_total: b.minted_total,
            audit_root: b.audit_root,
            envelope,
        })
    }

    /// Tally a proposal's ballots under quadratic voting (R5.3).
    #[must_use]
    pub fn tally(&self, ballots: &[Ballot]) -> Tally {
        let mut for_power = 0u128;
        let mut against_power = 0u128;
        for ballot in ballots {
            let power = isqrt(ballot.token_weight);
            if ballot.support {
                for_power = for_power.saturating_add(power);
            } else {
                against_power = against_power.saturating_add(power);
            }
        }
        Tally {
            for_power,
            against_power,
        }
    }

    /// Enact a passed proposal as an SLH-DSA-signed policy update (producer side
    /// of the downward gate 5→2/4). Enforces R5.4 before signing.
    pub fn enact(&self, proposal: &Proposal, tally: Tally) -> CputResult<PolicyUpdate> {
        if !tally.passed() {
            return Err(CputError::RuleViolation {
                rule: "R5.3",
                detail: "cannot enact a proposal that did not pass".into(),
            });
        }
        if proposal.mint_ceiling.0 == 0 && !proposal.circuit_breaker_engaged {
            return Err(CputError::RuleViolation {
                rule: "R5.4",
                detail: "zero ceiling requires the circuit breaker".into(),
            });
        }
        let body = PolicyUpdateBody {
            effective_epoch: proposal.effective_epoch,
            mint_ceiling: proposal.mint_ceiling,
            circuit_breaker_engaged: proposal.circuit_breaker_engaged,
            approved_algorithms: proposal.approved_algorithms.clone(),
        };
        Signed::seal_slhdsa(&self.governance_key, body)
    }
}
