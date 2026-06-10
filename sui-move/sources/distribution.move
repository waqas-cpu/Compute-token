/// # distribution — newly-minted supply splitter (70/15/10/5)
///
/// Implements R4.2: minted epoch totals split into provider, oracle, treasury,
/// and burn-reserve shares that sum exactly to the total.
module cput::distribution {
    use cput::cput::{Self, ProtocolState};
    use cput::policy::{Self, Distribution};

    /// Mint the four-way split to configured pool addresses.
    public(package) fun distribute(
        state: &mut ProtocolState,
        total: u64,
        ctx: &mut TxContext,
    ): Distribution {
        let split = policy::compute_distribution(total);
        let provider_pool = cput::provider_pool(state);
        let oracle_pool = cput::oracle_pool(state);
        let treasury_pool = cput::treasury_pool(state);
        let burn_reserve_pool = cput::burn_reserve_pool(state);
        cput::mint_to(state, policy::providers(&split), provider_pool, ctx);
        cput::mint_to(state, policy::oracle(&split), oracle_pool, ctx);
        cput::mint_to(state, policy::treasury(&split), treasury_pool, ctx);
        cput::mint_to(state, policy::burn_reserve(&split), burn_reserve_pool, ctx);
        split
    }
}
