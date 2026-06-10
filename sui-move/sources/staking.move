/// # staking — operator/provider stake & slashing
module cput::staking {
    use sui::table::{Self, Table};
    use sui::coin::{Self, Coin};
    use sui::event;
    use cput::cput::{Self, AdminCap, CPUT, ProtocolState};

    const E_INSUFFICIENT_STAKE: u64 = 1;
    const E_ZERO_AMOUNT: u64 = 2;

    public struct StakeBook has key {
        id: UID,
        stake: Table<address, u64>,
        escrow: Coin<CPUT>,
    }

    public struct Staked has copy, drop {
        who: address,
        amount: u64,
    }

    public struct Unstaked has copy, drop {
        who: address,
        amount: u64,
    }

    public struct Slashed has copy, drop {
        who: address,
        amount: u64,
    }

    public entry fun initialize(_admin: &AdminCap, ctx: &mut TxContext) {
        let book = StakeBook {
            id: object::new(ctx),
            stake: table::new(ctx),
            escrow: coin::zero<CPUT>(ctx),
        };
        transfer::share_object(book);
    }

    fun add_stake(book: &mut StakeBook, who: address, amount: u64) {
        if (table::contains(&book.stake, who)) {
            let s = table::borrow_mut(&mut book.stake, who);
            *s = *s + amount;
        } else {
            table::add(&mut book.stake, who, amount);
        }
    }

    fun remove_stake(book: &mut StakeBook, who: address, amount: u64) {
        assert!(table::contains(&book.stake, who), E_INSUFFICIENT_STAKE);
        let s = table::borrow_mut(&mut book.stake, who);
        assert!(*s >= amount, E_INSUFFICIENT_STAKE);
        *s = *s - amount;
    }

    /// Post CPUT stake into protocol escrow.
    public entry fun stake(book: &mut StakeBook, coin: Coin<CPUT>, ctx: &mut TxContext) {
        let who = ctx.sender();
        let amount = coin::value(&coin);
        assert!(amount > 0, E_ZERO_AMOUNT);
        coin::join(&mut book.escrow, coin);
        add_stake(book, who, amount);
        event::emit(Staked { who, amount });
    }

    /// Withdraw escrowed stake back to the staker.
    public entry fun unstake(book: &mut StakeBook, amount: u64, ctx: &mut TxContext) {
        let who = ctx.sender();
        assert!(amount > 0, E_ZERO_AMOUNT);
        remove_stake(book, who, amount);
        let withdrawn = coin::split(&mut book.escrow, amount, ctx);
        cput::transfer_coin(withdrawn, who);
        event::emit(Unstaked { who, amount });
    }

    /// Slash offender stake; slashed CPUT is burned (reduces supply).
    public entry fun slash(
        _admin: &AdminCap,
        book: &mut StakeBook,
        protocol: &mut ProtocolState,
        offender: address,
        amount: u64,
        ctx: &mut TxContext,
    ) {
        assert!(amount > 0, E_ZERO_AMOUNT);
        remove_stake(book, offender, amount);
        let slashed = coin::split(&mut book.escrow, amount, ctx);
        cput::burn_coin(protocol, slashed);
        event::emit(Slashed { who: offender, amount });
    }

    public fun stake_of(book: &StakeBook, who: address): u64 {
        if (table::contains(&book.stake, who)) {
            *table::borrow(&book.stake, who)
        } else {
            0
        }
    }
}
