/// # distribution — newly-minted supply splitter (70/15/10/5)
///
/// Implements Gate 4→5's economic invariant on-chain: a minted epoch total is
/// split into provider (70%), oracle (15%), treasury (10%) and burn-reserve
/// (5%) shares that sum *exactly* to the total (the rounding remainder is
/// folded into the provider share, matching the off-chain engine). Each share
/// is minted directly to its pool account.
module cput::distribution {
    use cput::cput_token;

    friend cput::minting;

    /// Distribution split (mirror `cput_core::policy`, basis points).
    const SPLIT_PROVIDERS_BPS: u128 = 7000;
    const SPLIT_ORACLE_BPS: u128 = 1500;
    const SPLIT_TREASURY_BPS: u128 = 1000;
    const SPLIT_BURN_RESERVE_BPS: u128 = 500;

    /// The computed four-way split of a minted total.
    struct Split has drop, copy, store {
        providers: u128,
        oracle: u128,
        treasury: u128,
        burn_reserve: u128,
    }

    fun apply_bps(total: u128, bps: u128): u128 {
        total * bps / 10000
    }

    /// Compute the split, folding the rounding remainder into providers so the
    /// shares sum to exactly `total` (matches the off-chain Gate 4→5 check).
    public fun compute_split(total: u128): Split {
        let oracle = apply_bps(total, SPLIT_ORACLE_BPS);
        let treasury = apply_bps(total, SPLIT_TREASURY_BPS);
        let burn_reserve = apply_bps(total, SPLIT_BURN_RESERVE_BPS);
        let providers = total - oracle - treasury - burn_reserve;
        Split { providers, oracle, treasury, burn_reserve }
    }

    /// Mint the split shares to their respective pools (friend: minting).
    public(friend) fun distribute(
        total: u128,
        provider_pool: address,
        oracle_pool: address,
        treasury_pool: address,
        burn_reserve_pool: address,
    ): Split {
        let split = compute_split(total);
        cput_token::mint(provider_pool, split.providers);
        cput_token::mint(oracle_pool, split.oracle);
        cput_token::mint(treasury_pool, split.treasury);
        cput_token::mint(burn_reserve_pool, split.burn_reserve);
        split
    }

    public fun providers(s: &Split): u128 { s.providers }
    public fun oracle(s: &Split): u128 { s.oracle }
    public fun treasury(s: &Split): u128 { s.treasury }
    public fun burn_reserve(s: &Split): u128 { s.burn_reserve }
}
