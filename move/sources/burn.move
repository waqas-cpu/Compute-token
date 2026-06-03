/// # burn — demand-side burn market
///
/// Routes access fees paid in $CPUT: a fixed fraction is permanently burned
/// (deflationary pressure), the rest is split between providers and the
/// treasury. Mirrors the off-chain `cput-layer4-tokenomics` burn market
/// (20% burned / 75% providers / 5% treasury) and reduces total supply by the
/// burned portion.
module cput::burn {
    use std::signer;
    use aptos_framework::event;
    use cput::cput_token;

    /// Burn-market split (mirror `cput_core::policy`, basis points).
    const BURN_FEE_BURNED_BPS: u128 = 2000;
    const BURN_FEE_PROVIDER_BPS: u128 = 7500;
    const BURN_FEE_TREASURY_BPS: u128 = 500;

    #[event]
    struct FeeRouted has drop, store {
        payer: address,
        burned: u128,
        providers: u128,
        treasury: u128,
    }

    fun apply_bps(total: u128, bps: u128): u128 {
        total * bps / 10000
    }

    /// Pay an access fee: burn the deflationary share and route the remainder
    /// to the provider and treasury pools. The remainder after burn+providers
    /// goes to treasury so the parts sum to the fee exactly.
    public entry fun pay_access_fee(
        payer: &signer,
        fee: u128,
        provider_pool: address,
        treasury_pool: address,
    ) {
        let payer_addr = signer::address_of(payer);
        let burned = apply_bps(fee, BURN_FEE_BURNED_BPS);
        let providers = apply_bps(fee, BURN_FEE_PROVIDER_BPS);
        let treasury = fee - burned - providers;

        // Burned tokens leave circulation; the rest moves to the pools.
        cput_token::burn(payer_addr, burned);
        cput_token::internal_transfer(payer_addr, provider_pool, providers);
        cput_token::internal_transfer(payer_addr, treasury_pool, treasury);

        event::emit(FeeRouted { payer: payer_addr, burned, providers, treasury });
    }

    // The fraction (bps) of a fee that is burned.
    #[view]
    public fun burned_bps(): u128 { BURN_FEE_BURNED_BPS }
}
