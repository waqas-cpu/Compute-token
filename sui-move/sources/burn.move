/// # burn — demand-side burn market (utility + payment token economics)
module cput::burn {
    use sui::coin::{Self, Coin};
    use sui::event;
    use cput::cput::{Self, CPUT, ProtocolState};
    use cput::policy;

    const E_ZERO_FEE: u64 = 1;

    public struct FeeRouted has copy, drop {
        payer: address,
        fee: u64,
        burned: u64,
        providers: u64,
        treasury: u64,
    }

    /// Pay an access fee in CPUT — burns supply and routes the rest to pools.
    public entry fun pay_access_fee(
        protocol: &mut ProtocolState,
        mut fee: Coin<CPUT>,
        ctx: &mut TxContext,
    ) {
        cput::assert_pools_configured(protocol);
        let payer = ctx.sender();
        let fee_amount = coin::value(&fee);
        assert!(fee_amount > 0, E_ZERO_FEE);

        let routing = policy::compute_fee_routing(fee_amount);
        let provider_pool = cput::provider_pool(protocol);
        let treasury_pool = cput::treasury_pool(protocol);

        let burn_coin = coin::split(&mut fee, policy::fee_burned(&routing), ctx);
        cput::burn_coin(protocol, burn_coin);

        let provider_coin = coin::split(&mut fee, policy::fee_providers(&routing), ctx);
        cput::transfer_coin(provider_coin, provider_pool);

        cput::transfer_coin(fee, treasury_pool);

        event::emit(FeeRouted {
            payer,
            fee: fee_amount,
            burned: policy::fee_burned(&routing),
            providers: policy::fee_providers(&routing),
            treasury: policy::fee_treasury(&routing),
        });
    }

    public fun burned_bps(): u64 {
        policy::apply_bps(10_000, 2000)
    }
}
