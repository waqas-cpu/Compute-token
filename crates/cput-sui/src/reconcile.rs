//! Off-chain ↔ on-chain economic reconciliation.

use crate::types::OnChainEpochMinted;
use cput_core::{CputError, CputResult};
use cput_gates::contracts::SettlementReceiptBody;

/// Compare an off-chain settlement receipt with an on-chain `EpochMinted` event.
pub fn reconcile_epoch_mint(
    receipt: &SettlementReceiptBody,
    on_chain: &OnChainEpochMinted,
) -> CputResult<()> {
    if receipt.epoch.0 != on_chain.epoch {
        return Err(CputError::Chain(format!(
            "epoch mismatch: off-chain {} vs on-chain {}",
            receipt.epoch.0, on_chain.epoch
        )));
    }
    let checks = [
        ("total", receipt.minted_total.0, on_chain.total as u128),
        ("providers", receipt.providers.0, on_chain.providers as u128),
        ("oracle", receipt.oracle.0, on_chain.oracle as u128),
        ("treasury", receipt.treasury.0, on_chain.treasury as u128),
        ("burn_reserve", receipt.burn_reserve.0, on_chain.burn_reserve as u128),
    ];
    for (label, off, on) in checks {
        if off != on {
            return Err(CputError::Chain(format!(
                "{label} mismatch for epoch {}: off-chain {off} vs on-chain {on}",
                receipt.epoch.0
            )));
        }
    }
    Ok(())
}

/// Compare off-chain fee routing with on-chain `FeeRouted` event fields.
pub fn reconcile_fee_routed(
    fee: u64,
    burned: u64,
    providers: u64,
    treasury: u64,
    on_fee: u64,
    on_burned: u64,
    on_providers: u64,
    on_treasury: u64,
) -> CputResult<()> {
    if fee != on_fee {
        return Err(CputError::Chain(format!(
            "fee mismatch: off-chain {fee} vs on-chain {on_fee}"
        )));
    }
    if burned != on_burned || providers != on_providers || treasury != on_treasury {
        return Err(CputError::Chain(format!(
            "fee routing mismatch: off ({burned}/{providers}/{treasury}) vs on ({on_burned}/{on_providers}/{on_treasury})"
        )));
    }
    Ok(())
}
