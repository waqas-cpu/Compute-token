/// # policy — shared economic constants (mirrors `cput_core::policy`)
///
/// Single source of truth for the distribution split (mint) and burn-market
/// (payment/utility fee) formulas used on- and off-chain.
module cput::policy {
    // --- Mint distribution split (must sum to 10_000 bps). ---
    const SPLIT_PROVIDERS_BPS: u64 = 7000;
    const SPLIT_ORACLE_BPS: u64 = 1500;
    const SPLIT_TREASURY_BPS: u64 = 1000;
    const SPLIT_BURN_RESERVE_BPS: u64 = 500;

    // --- Demand-side burn market / payment fee routing. ---
    const BURN_FEE_BURNED_BPS: u64 = 2000;
    const BURN_FEE_PROVIDER_BPS: u64 = 7500;

    // --- Elastic supply governor. ---
    const UTIL_LOW_BPS: u64 = 6000;
    const UTIL_HIGH_BPS: u64 = 9000;
    const CEILING_DECREASE_BPS: u64 = 500;
    const CEILING_INCREASE_BPS: u64 = 300;

    // --- Oracle DON parameters. ---
    const ORACLE_SET_SIZE: u64 = 9;
    const ORACLE_THRESHOLD: u64 = 5;

    // --- AI agentic minting quorum (Layer 2). ---
    const AGENT_QUORUM_SIZE: u64 = 3;
    const CONSENSUS_TOLERANCE_BPS: u64 = 200;
    /// Default emission: 1 base-unit CPUT per verified GFLOP (mirrors off-chain).
    const DEFAULT_TOKENS_PER_GFLOP: u64 = 1;

    /// Four-way split of a newly minted epoch total (R4.2).
    public struct Distribution has copy, drop, store {
        providers: u64,
        oracle: u64,
        treasury: u64,
        burn_reserve: u64,
    }

    /// Result of routing a utility/payment access fee (R4.4).
    public struct FeeRouting has copy, drop, store {
        burned: u64,
        providers: u64,
        treasury: u64,
    }

    public fun apply_bps(total: u64, bps: u64): u64 {
        total * bps / 10_000
    }

    /// Compute the mint split; rounding remainder folds into providers.
    public fun compute_distribution(total: u64): Distribution {
        let oracle = apply_bps(total, SPLIT_ORACLE_BPS);
        let treasury = apply_bps(total, SPLIT_TREASURY_BPS);
        let burn_reserve = apply_bps(total, SPLIT_BURN_RESERVE_BPS);
        let providers = total - oracle - treasury - burn_reserve;
        Distribution { providers, oracle, treasury, burn_reserve }
    }

    /// Route an access fee: 20% burned, 75% providers, remainder treasury.
    public fun compute_fee_routing(fee: u64): FeeRouting {
        let burned = apply_bps(fee, BURN_FEE_BURNED_BPS);
        let providers = apply_bps(fee, BURN_FEE_PROVIDER_BPS);
        let treasury = fee - burned - providers;
        FeeRouting { burned, providers, treasury }
    }

    /// Recommend the next per-epoch ceiling from a TWAP utilisation signal (R4.5).
    public fun recommend_ceiling(current: u64, twap_util_bps: u64): u64 {
        if (twap_util_bps < UTIL_LOW_BPS) {
            current - apply_bps(current, CEILING_DECREASE_BPS)
        } else if (twap_util_bps > UTIL_HIGH_BPS) {
            current + apply_bps(current, CEILING_INCREASE_BPS)
        } else {
            current
        }
    }

    public fun providers(d: &Distribution): u64 { d.providers }
    public fun oracle(d: &Distribution): u64 { d.oracle }
    public fun treasury(d: &Distribution): u64 { d.treasury }
    public fun burn_reserve(d: &Distribution): u64 { d.burn_reserve }

    public fun fee_burned(r: &FeeRouting): u64 { r.burned }
    public fun fee_providers(r: &FeeRouting): u64 { r.providers }
    public fun fee_treasury(r: &FeeRouting): u64 { r.treasury }

    public fun oracle_threshold(): u64 { ORACLE_THRESHOLD }
    public fun oracle_set_size(): u64 { ORACLE_SET_SIZE }
    public fun agent_quorum_size(): u64 { AGENT_QUORUM_SIZE }
    public fun consensus_tolerance_bps(): u64 { CONSENSUS_TOLERANCE_BPS }
    public fun default_tokens_per_gflop(): u64 { DEFAULT_TOKENS_PER_GFLOP }
    public fun util_low_bps(): u64 { UTIL_LOW_BPS }
    public fun util_high_bps(): u64 { UTIL_HIGH_BPS }

    /// Basis-point spread between min and max agent proposals (R2.2).
    public fun spread_bps(min: u64, max: u64): u64 {
        if (min == 0) return 0;
        (max - min) * 10_000 / min
    }

    /// Integer average of three agent proposals (minting quorum).
    public fun consensus_total(a: u64, b: u64, c: u64): u64 {
        (a + b + c) / 3
    }
}
