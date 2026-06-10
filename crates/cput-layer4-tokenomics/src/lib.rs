//! # cput-layer4-tokenomics — VERTICAL LAYER 4
//!
//! The off-chain tokenomics engine mirror. It consumes the agent
//! [`MintInstruction`] (admitting it through **Gate 2→4**), splits the minted
//! supply per the protocol distribution policy, records the mint in the
//! horizontal audit log, and emits the ML-DSA-signed [`SettlementReceipt`]
//! consumed by Layer 5 (**Gate 4→5** producer side). It also implements the
//! demand-side burn market and the elastic-supply governor.
//!
//! ## Layer rules (invariants this layer guarantees)
//!
//! - **R4.1 Ceiling enforcement.** No mint exceeds the current per-epoch
//!   ceiling (re-checked at Gate 2→4).
//! - **R4.2 Conservative split.** The four distribution shares are computed by
//!   the exact policy formula and always sum to the minted total.
//! - **R4.3 Auditability.** Every mint and burn is appended to the audit log
//!   before the receipt is signed; the receipt commits to the resulting root.
//! - **R4.4 Burn market.** Access fees split into burn/provider/treasury per
//!   policy; the burned portion reduces circulating supply.
//! - **R4.5 Elastic ceiling.** The governor moves the ceiling within bounded
//!   steps using a smoothed (TWAP) utilisation signal.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use cput_audit::{AuditAction, AuditLog};
use cput_core::ids::EpochId;
use cput_core::units::{Bps, Gflops, TokenAmount};
use cput_core::{policy, CputError, CputResult};
use cput_gates::contracts::{MintInstruction, SettlementReceipt, SettlementReceiptBody};
use cput_gates::gate::{admit_mint_instruction, GateConfig};
use cput_pqc::envelope::Signed;
use cput_pqc::mldsa::MlDsaKeypair;
use cput_zk::ProofBackend;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// The four-way split of a minted total (R4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Distribution {
    /// Compute-provider share.
    pub providers: TokenAmount,
    /// Oracle-staker share.
    pub oracle: TokenAmount,
    /// DAO-treasury share.
    pub treasury: TokenAmount,
    /// Burn-reserve share.
    pub burn_reserve: TokenAmount,
}

impl Distribution {
    /// Compute the policy split, folding the rounding remainder into providers
    /// so the shares sum to exactly `minted_total` (matches Gate 4→5).
    #[must_use]
    pub fn of(minted_total: TokenAmount) -> Self {
        let oracle = minted_total.apply_bps(policy::SPLIT_ORACLE_BPS);
        let treasury = minted_total.apply_bps(policy::SPLIT_TREASURY_BPS);
        let burn_reserve = minted_total.apply_bps(policy::SPLIT_BURN_RESERVE_BPS);
        let providers = TokenAmount(
            minted_total
                .0
                .saturating_sub(oracle.0)
                .saturating_sub(treasury.0)
                .saturating_sub(burn_reserve.0),
        );
        Self {
            providers,
            oracle,
            treasury,
            burn_reserve,
        }
    }
}

/// The result of routing an access fee through the demand-side burn market.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeRouting {
    /// Amount permanently burned (reduces supply).
    pub burned: TokenAmount,
    /// Amount routed to providers.
    pub providers: TokenAmount,
    /// Amount routed to the treasury.
    pub treasury: TokenAmount,
}

/// The off-chain tokenomics engine state mirror.
pub struct TokenomicsEngine<'a, B: ProofBackend> {
    cfg: GateConfig<'a, B>,
    signer: MlDsaKeypair,
    expected_agent_key: Vec<u8>,
    /// Current per-epoch mint ceiling (governed by Layer 5 / the governor).
    pub mint_ceiling: TokenAmount,
    /// Circulating supply mirror.
    pub circulating_supply: TokenAmount,
    /// Accumulated burn reserve.
    pub burn_reserve: TokenAmount,
    audit: AuditLog,
}

fn canonical<T: Serialize>(v: &T) -> CputResult<Vec<u8>> {
    serde_json::to_vec(v).map_err(|e| CputError::Codec(format!("l4 canonical encode: {e}")))
}

impl<'a, B: ProofBackend> TokenomicsEngine<'a, B> {
    /// Construct the engine bound to the registered coordinating agent key.
    pub fn new(
        cfg: GateConfig<'a, B>,
        signer: MlDsaKeypair,
        expected_agent_key: Vec<u8>,
        initial_ceiling: TokenAmount,
    ) -> Self {
        Self {
            cfg,
            signer,
            expected_agent_key,
            mint_ceiling: initial_ceiling,
            circulating_supply: TokenAmount(0),
            burn_reserve: TokenAmount(0),
            audit: AuditLog::new(),
        }
    }

    /// The engine's settlement-signing public key (checked at Gate 4→5).
    #[must_use]
    pub fn signer_key(&self) -> Vec<u8> {
        self.signer.public_bytes()
    }

    /// Current audit-log Merkle root.
    #[must_use]
    pub fn audit_root(&self) -> [u8; 32] {
        self.audit.root()
    }

    /// Settle an epoch: admit the mint instruction (Gate 2→4), split, record to
    /// the audit log, and sign the settlement receipt (R4.1–R4.3).
    pub fn settle(
        &mut self,
        verified_gflops: Gflops,
        instruction: &MintInstruction,
    ) -> CputResult<SettlementReceipt> {
        // R4.1 — re-check the instruction against the live ceiling at Gate 2→4.
        admit_mint_instruction(
            &self.cfg,
            &self.expected_agent_key,
            verified_gflops,
            self.mint_ceiling,
            instruction,
        )?;

        let body = &instruction.signed.body;
        let dist = Distribution::of(body.total); // R4.2

        // R4.3 — record the mint before signing the receipt.
        let payload_hash: [u8; 32] = Sha256::digest(canonical(body)?).into();
        self.audit.append(
            body.epoch,
            AuditAction::Mint {
                total: body.total.0,
            },
            payload_hash,
        );
        self.circulating_supply = self.circulating_supply.saturating_add(body.total);
        self.burn_reserve = self.burn_reserve.saturating_add(dist.burn_reserve);

        let receipt_body = SettlementReceiptBody {
            epoch: body.epoch,
            minted_total: body.total,
            providers: dist.providers,
            oracle: dist.oracle,
            treasury: dist.treasury,
            burn_reserve: dist.burn_reserve,
            audit_root: self.audit.root(),
        };
        Signed::seal_mldsa(&self.signer, receipt_body)
    }

    /// Route a demand-side access fee through the burn market (R4.4).
    pub fn process_access_fee(
        &mut self,
        epoch: EpochId,
        fee: TokenAmount,
    ) -> CputResult<FeeRouting> {
        let burned = fee.apply_bps(policy::BURN_FEE_BURNED_BPS);
        let providers = fee.apply_bps(policy::BURN_FEE_PROVIDER_BPS);
        let treasury = TokenAmount(fee.0.saturating_sub(burned.0).saturating_sub(providers.0));

        // Burned tokens leave circulation.
        self.circulating_supply = TokenAmount(self.circulating_supply.0.saturating_sub(burned.0));
        let payload_hash: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(b"cput.l4.burn.v1");
            h.update(fee.0.to_le_bytes());
            h.finalize().into()
        };
        self.audit
            .append(epoch, AuditAction::Burn { amount: burned.0 }, payload_hash);
        Ok(FeeRouting {
            burned,
            providers,
            treasury,
        })
    }

    /// Recommend the next epoch's mint ceiling from a smoothed (TWAP)
    /// utilisation signal, in bounded steps (R4.5). Pure — does not mutate
    /// state; the new value is enacted via a Layer 5 policy update.
    #[must_use]
    pub fn recommend_ceiling(&self, twap_util_bps: Bps) -> TokenAmount {
        let current = self.mint_ceiling.0;
        if twap_util_bps.0 < policy::UTIL_LOW_BPS.0 {
            // Oversupply risk — contract the ceiling.
            let delta = current.saturating_mul(u128::from(policy::CEILING_DECREASE_BPS.0)) / 10_000;
            TokenAmount(current.saturating_sub(delta))
        } else if twap_util_bps.0 > policy::UTIL_HIGH_BPS.0 {
            // Scarcity — expand the ceiling within the capped step.
            let delta = current.saturating_mul(u128::from(policy::CEILING_INCREASE_BPS.0)) / 10_000;
            TokenAmount(current.saturating_add(delta))
        } else {
            self.mint_ceiling
        }
    }

    /// Enact a new ceiling (called when a Layer 5 policy update is admitted).
    pub fn set_ceiling(&mut self, ceiling: TokenAmount) {
        self.mint_ceiling = ceiling;
    }

    /// Sync the live ceiling from a Sui governance intent (downward gate 5→2/4).
    pub fn sync_ceiling_from_chain(
        &mut self,
        mint_ceiling: u64,
        _circuit_breaker_engaged: bool,
    ) -> CputResult<()> {
        self.mint_ceiling = TokenAmount(u128::from(mint_ceiling));
        Ok(())
    }
}
