/// # staking — operator/provider stake & slashing
///
/// Oracle operators and compute providers post $CPUT stake as collateral.
/// Stake is escrowed in the protocol account; misbehaviour (proven via the
/// dispute path) is punished by slashing — burning a portion of the offender's
/// escrowed stake. Mirrors the security-deposit model that backs the oracle
/// threshold and attestation honesty assumptions.
module cput::staking {
    use std::signer;
    use aptos_framework::table::{Self, Table};
    use aptos_framework::event;
    use cput::cput_token;

    /// Caller is not the protocol admin/governance.
    const E_NOT_ADMIN: u64 = 1;
    /// Staker has no (or insufficient) escrowed stake.
    const E_INSUFFICIENT_STAKE: u64 = 2;

    /// Escrowed stake balances.
    struct StakeBook has key {
        stake: Table<address, u128>,
    }

    #[event]
    struct Staked has drop, store { who: address, amount: u128 }
    #[event]
    struct Unstaked has drop, store { who: address, amount: u128 }
    #[event]
    struct Slashed has drop, store { who: address, amount: u128 }

    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        move_to(admin, StakeBook { stake: table::new<address, u128>() });
    }

    fun add_stake(book: &mut StakeBook, who: address, amount: u128) {
        if (table::contains(&book.stake, who)) {
            let s = table::borrow_mut(&mut book.stake, who);
            *s = *s + amount;
        } else {
            table::add(&mut book.stake, who, amount);
        }
    }

    fun remove_stake(book: &mut StakeBook, who: address, amount: u128) {
        assert!(table::contains(&book.stake, who), E_INSUFFICIENT_STAKE);
        let s = table::borrow_mut(&mut book.stake, who);
        assert!(*s >= amount, E_INSUFFICIENT_STAKE);
        *s = *s - amount;
    }

    /// Post stake: escrow `amount` from the caller into the protocol account.
    public entry fun stake(staker: &signer, amount: u128) acquires StakeBook {
        let who = signer::address_of(staker);
        cput_token::internal_transfer(who, @cput, amount);
        let book = borrow_global_mut<StakeBook>(@cput);
        add_stake(book, who, amount);
        event::emit(Staked { who, amount });
    }

    /// Withdraw stake back to the staker.
    public entry fun unstake(staker: &signer, amount: u128) acquires StakeBook {
        let who = signer::address_of(staker);
        let book = borrow_global_mut<StakeBook>(@cput);
        remove_stake(book, who, amount);
        cput_token::internal_transfer(@cput, who, amount);
        event::emit(Unstaked { who, amount });
    }

    /// Slash `amount` of `offender`'s escrowed stake (admin/governance gated).
    /// Slashed stake is burned, reducing total supply.
    public entry fun slash(admin: &signer, offender: address, amount: u128) acquires StakeBook {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        let book = borrow_global_mut<StakeBook>(@cput);
        remove_stake(book, offender, amount);
        cput_token::burn(@cput, amount);
        event::emit(Slashed { who: offender, amount });
    }

    #[view]
    public fun stake_of(who: address): u128 acquires StakeBook {
        let book = borrow_global<StakeBook>(@cput);
        if (table::contains(&book.stake, who)) { *table::borrow(&book.stake, who) } else { 0 }
    }
}
