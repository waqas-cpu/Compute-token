/// # governor — elastic-supply governor (per-epoch mint ceiling)
///
/// Holds the live per-epoch mint ceiling and the anti-inflation circuit
/// breaker. The ceiling is moved within bounded steps from a smoothed (TWAP)
/// utilisation signal; governance enacts the new value through a policy update
/// (the downward gate 5→2/4). `minting` reads the ceiling to cap each epoch's
/// mint.
module cput::governor {
    use std::signer;
    use aptos_framework::event;

    friend cput::governance;
    friend cput::minting;

    /// Utilisation thresholds & step caps (mirror `cput_core::policy`, bps).
    const UTIL_LOW_BPS: u64 = 6000;
    const UTIL_HIGH_BPS: u64 = 9000;
    const CEILING_DECREASE_BPS: u128 = 500;
    const CEILING_INCREASE_BPS: u128 = 300;

    /// Caller is not the protocol admin.
    const E_NOT_ADMIN: u64 = 1;
    /// A zero ceiling is only valid with the circuit breaker engaged.
    const E_UNSAFE_CEILING: u64 = 2;

    struct Governor has key {
        mint_ceiling: u128,
        circuit_breaker_engaged: bool,
    }

    #[event]
    struct CeilingUpdated has drop, store { mint_ceiling: u128, circuit_breaker_engaged: bool }

    /// Initialise the governor with a genesis ceiling.
    public entry fun initialize(admin: &signer, initial_ceiling: u128) {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        move_to(admin, Governor { mint_ceiling: initial_ceiling, circuit_breaker_engaged: false });
    }

    // Recommend the next ceiling from a smoothed utilisation signal (pure).
    #[view]
    public fun recommend_ceiling(current: u128, twap_util_bps: u64): u128 {
        if (twap_util_bps < UTIL_LOW_BPS) {
            current - (current * CEILING_DECREASE_BPS / 10000)
        } else if (twap_util_bps > UTIL_HIGH_BPS) {
            current + (current * CEILING_INCREASE_BPS / 10000)
        } else {
            current
        }
    }

    /// Enact a new ceiling (friend: governance, via an admitted policy update).
    public(friend) fun set_ceiling(
        mint_ceiling: u128,
        circuit_breaker_engaged: bool,
    ) acquires Governor {
        assert!(mint_ceiling > 0 || circuit_breaker_engaged, E_UNSAFE_CEILING);
        let g = borrow_global_mut<Governor>(@cput);
        g.mint_ceiling = mint_ceiling;
        g.circuit_breaker_engaged = circuit_breaker_engaged;
        event::emit(CeilingUpdated { mint_ceiling, circuit_breaker_engaged });
    }

    /// The current per-epoch mint ceiling (friend: minting).
    public(friend) fun current_ceiling(): u128 acquires Governor {
        borrow_global<Governor>(@cput).mint_ceiling
    }

    #[view]
    public fun ceiling(): u128 acquires Governor {
        borrow_global<Governor>(@cput).mint_ceiling
    }

    #[view]
    public fun breaker_engaged(): bool acquires Governor {
        borrow_global<Governor>(@cput).circuit_breaker_engaged
    }
}
