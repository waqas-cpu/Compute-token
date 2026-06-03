/// # cput_token — the $CPUT security token (ERC-1400 style)
///
/// A compliance-gated, mintable/burnable balance ledger. Minting is restricted
/// to the `minting` module, burning to the `burn` module, and stake custody to
/// the `staking` module (via Move `friend` visibility), so supply can only
/// change through the audited protocol paths. Transfers consult the
/// `compliance` registry, giving ERC-1400 transfer-restriction semantics.
module cput::cput_token {
    use std::signer;
    use aptos_framework::table::{Self, Table};
    use aptos_framework::event;
    use cput::compliance;

    friend cput::minting;
    friend cput::distribution;
    friend cput::burn;
    friend cput::staking;

    /// 8 on-chain decimals (mirrors `TokenAmount` base units off-chain).
    const DECIMALS: u8 = 8;

    /// Caller is not the protocol admin.
    const E_NOT_ADMIN: u64 = 1;
    /// Insufficient balance for the operation.
    const E_INSUFFICIENT: u64 = 2;
    /// Token already initialised.
    const E_ALREADY_INIT: u64 = 3;

    /// Global token ledger.
    struct TokenState has key {
        total_supply: u128,
        balances: Table<address, u128>,
    }

    #[event]
    struct Minted has drop, store { to: address, amount: u128 }
    #[event]
    struct Burned has drop, store { from: address, amount: u128 }
    #[event]
    struct Transferred has drop, store { from: address, to: address, amount: u128 }

    /// Initialise the token ledger under the admin account.
    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        assert!(!exists<TokenState>(@cput), E_ALREADY_INIT);
        move_to(admin, TokenState { total_supply: 0, balances: table::new<address, u128>() });
    }

    fun credit(state: &mut TokenState, to: address, amount: u128) {
        if (table::contains(&state.balances, to)) {
            let b = table::borrow_mut(&mut state.balances, to);
            *b = *b + amount;
        } else {
            table::add(&mut state.balances, to, amount);
        }
    }

    fun debit(state: &mut TokenState, from: address, amount: u128) {
        assert!(table::contains(&state.balances, from), E_INSUFFICIENT);
        let b = table::borrow_mut(&mut state.balances, from);
        assert!(*b >= amount, E_INSUFFICIENT);
        *b = *b - amount;
    }

    /// Mint `amount` to `to` (only the minting / distribution modules).
    public(friend) fun mint(to: address, amount: u128) acquires TokenState {
        let state = borrow_global_mut<TokenState>(@cput);
        credit(state, to, amount);
        state.total_supply = state.total_supply + amount;
        event::emit(Minted { to, amount });
    }

    /// Burn `amount` from `from` (only the burn / staking modules).
    public(friend) fun burn(from: address, amount: u128) acquires TokenState {
        let state = borrow_global_mut<TokenState>(@cput);
        debit(state, from, amount);
        state.total_supply = state.total_supply - amount;
        event::emit(Burned { from, amount });
    }

    /// Move tokens between custodial protocol accounts (friend modules) without
    /// the compliance gate — used for stake escrow and reward routing.
    public(friend) fun internal_transfer(from: address, to: address, amount: u128) acquires TokenState {
        let state = borrow_global_mut<TokenState>(@cput);
        debit(state, from, amount);
        credit(state, to, amount);
        event::emit(Transferred { from, to, amount });
    }

    /// Compliance-gated holder transfer (ERC-1400 `transferWithData`).
    public entry fun transfer(from: &signer, to: address, amount: u128) acquires TokenState {
        let from_addr = signer::address_of(from);
        compliance::assert_can_transfer(from_addr, to);
        let state = borrow_global_mut<TokenState>(@cput);
        debit(state, from_addr, amount);
        credit(state, to, amount);
        event::emit(Transferred { from: from_addr, to, amount });
    }

    #[view]
    public fun balance_of(owner: address): u128 acquires TokenState {
        let state = borrow_global<TokenState>(@cput);
        if (table::contains(&state.balances, owner)) {
            *table::borrow(&state.balances, owner)
        } else { 0 }
    }

    #[view]
    public fun total_supply(): u128 acquires TokenState {
        borrow_global<TokenState>(@cput).total_supply
    }

    #[view]
    public fun decimals(): u8 { DECIMALS }
}
