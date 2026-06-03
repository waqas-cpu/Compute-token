/// # minting — on-chain Gate 2→4 (agent mint instruction)
///
/// Enforces the L2→L4 integration gate on-chain. A mint instruction is only
/// executed if:
///   * the epoch has an oracle-verified report (Gate 1→2 cleared),
///   * the requested total does not exceed the governor's live ceiling (R4.1),
///   * the epoch has not already been minted (replay protection),
/// after which the supply is split and minted through the `distribution`
/// module (which guarantees the shares sum to the total — R4.2).
///
/// The agent's ML-DSA signature and reasoning-proof bytes are checked by the
/// off-chain relayer / a future native; this module binds the instruction to
/// the verified epoch and enforces the economic invariants.
module cput::minting {
    use std::signer;
    use aptos_framework::table::{Self, Table};
    use aptos_framework::event;
    use cput::oracle_verifier;
    use cput::governor;
    use cput::distribution;

    /// Caller is not the protocol relayer/admin.
    const E_NOT_ADMIN: u64 = 1;
    /// No oracle-verified report for the epoch.
    const E_NO_REPORT: u64 = 2;
    /// Requested mint exceeds the live ceiling.
    const E_EXCEEDS_CEILING: u64 = 3;
    /// Epoch already minted.
    const E_ALREADY_MINTED: u64 = 4;
    /// Pools not configured.
    const E_NO_POOLS: u64 = 5;

    /// Destination pools for the distribution split.
    struct Pools has key {
        provider_pool: address,
        oracle_pool: address,
        treasury_pool: address,
        burn_reserve_pool: address,
    }

    /// Record of which epochs have been minted (replay protection).
    struct MintLog has key {
        minted: Table<u64, u128>,
    }

    #[event]
    struct EpochMinted has drop, store {
        epoch: u64,
        total: u128,
        providers: u128,
        oracle: u128,
        treasury: u128,
        burn_reserve: u128,
    }

    /// Configure the distribution pools and mint log.
    public entry fun initialize(
        admin: &signer,
        provider_pool: address,
        oracle_pool: address,
        treasury_pool: address,
        burn_reserve_pool: address,
    ) {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        move_to(admin, Pools { provider_pool, oracle_pool, treasury_pool, burn_reserve_pool });
        move_to(admin, MintLog { minted: table::new<u64, u128>() });
    }

    /// Execute an agent mint instruction for `epoch` (Gate 2→4). Relayer-gated.
    public entry fun execute_mint(
        admin: &signer,
        epoch: u64,
        total: u128,
    ) acquires Pools, MintLog {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        assert!(exists<Pools>(@cput), E_NO_POOLS);

        // Gate 1→2 must have cleared for this epoch.
        assert!(oracle_verifier::has_report(epoch), E_NO_REPORT);

        // R4.1 — ceiling enforcement.
        assert!(total <= governor::current_ceiling(), E_EXCEEDS_CEILING);

        // Replay protection — one mint per epoch.
        let log = borrow_global_mut<MintLog>(@cput);
        assert!(!table::contains(&log.minted, epoch), E_ALREADY_MINTED);
        table::add(&mut log.minted, epoch, total);

        // R4.2 — split & mint (shares sum to total by construction).
        let pools = borrow_global<Pools>(@cput);
        let split = distribution::distribute(
            total,
            pools.provider_pool,
            pools.oracle_pool,
            pools.treasury_pool,
            pools.burn_reserve_pool,
        );
        event::emit(EpochMinted {
            epoch,
            total,
            providers: distribution::providers(&split),
            oracle: distribution::oracle(&split),
            treasury: distribution::treasury(&split),
            burn_reserve: distribution::burn_reserve(&split),
        });
    }

    #[view]
    public fun is_minted(epoch: u64): bool acquires MintLog {
        table::contains(&borrow_global<MintLog>(@cput).minted, epoch)
    }
}
