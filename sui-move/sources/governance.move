/// # governance — DAO governance (quadratic voting) + downward Gate 5→2/4
module cput::governance {
    use sui::table::{Self, Table};
    use sui::coin::{Self, Coin};
    use sui::event;
    use cput::cput::{Self, AdminCap, CPUT, ProtocolState};

    const E_NO_PROPOSAL: u64 = 1;
    const E_DOUBLE_VOTE: u64 = 2;
    const E_FINALIZED: u64 = 3;
    const E_NOT_PASSED: u64 = 4;
    const E_UNSAFE_POLICY: u64 = 5;

    public struct Proposal has store {
        effective_epoch: u64,
        mint_ceiling: u64,
        circuit_breaker_engaged: bool,
        for_power: u64,
        against_power: u64,
        voters: vector<address>,
        finalized: bool,
    }

    public struct GovernanceState has key {
        id: UID,
        proposals: Table<u64, Proposal>,
        next_id: u64,
    }

    public struct ProposalCreated has copy, drop {
        id: u64,
        mint_ceiling: u64,
    }

    public struct Voted has copy, drop {
        id: u64,
        voter: address,
        power: u64,
        support: bool,
    }

    public struct Enacted has copy, drop {
        id: u64,
        mint_ceiling: u64,
    }

    public entry fun initialize(_admin: &AdminCap, ctx: &mut TxContext) {
        let state = GovernanceState {
            id: object::new(ctx),
            proposals: table::new(ctx),
            next_id: 0,
        };
        transfer::share_object(state);
    }

    /// Integer square root (floor) for quadratic voting weight (R5.3).
    public fun isqrt(n: u64): u64 {
        if (n < 2) return n;
        let mut x = n;
        let mut y = (x + 1) / 2;
        while (y < x) {
            x = y;
            y = (x + n / x) / 2;
        };
        x
    }

    public entry fun create_proposal(
        _admin: &AdminCap,
        gov: &mut GovernanceState,
        effective_epoch: u64,
        mint_ceiling: u64,
        circuit_breaker_engaged: bool,
    ) {
        let id = gov.next_id;
        gov.next_id = id + 1;
        table::add(&mut gov.proposals, id, Proposal {
            effective_epoch,
            mint_ceiling,
            circuit_breaker_engaged,
            for_power: 0,
            against_power: 0,
            voters: vector[],
            finalized: false,
        });
        event::emit(ProposalCreated { id, mint_ceiling });
    }

    /// Cast a quadratic vote; voting power = isqrt(coin value).
    public entry fun vote(
        gov: &mut GovernanceState,
        protocol: &mut ProtocolState,
        vote_coin: Coin<CPUT>,
        id: u64,
        support: bool,
        ctx: &mut TxContext,
    ) {
        let who = ctx.sender();
        assert!(table::contains(&gov.proposals, id), E_NO_PROPOSAL);
        let p = table::borrow_mut(&mut gov.proposals, id);
        assert!(!p.finalized, E_FINALIZED);
        assert!(!vector::contains(&p.voters, &who), E_DOUBLE_VOTE);

        let power = isqrt(coin::value(&vote_coin));
        if (support) {
            p.for_power = p.for_power + power;
        } else {
            p.against_power = p.against_power + power;
        };
        vector::push_back(&mut p.voters, who);
        event::emit(Voted { id, voter: who, power, support });
        cput::burn_coin(protocol, vote_coin);
    }

    /// Finalise a passed proposal and enact the new ceiling (Gate 5→2/4).
    public entry fun finalize(
        _admin: &AdminCap,
        gov: &mut GovernanceState,
        protocol: &mut ProtocolState,
        id: u64,
    ) {
        assert!(table::contains(&gov.proposals, id), E_NO_PROPOSAL);
        let p = table::borrow_mut(&mut gov.proposals, id);
        assert!(!p.finalized, E_FINALIZED);
        assert!(p.for_power > p.against_power, E_NOT_PASSED);
        assert!(p.mint_ceiling > 0 || p.circuit_breaker_engaged, E_UNSAFE_POLICY);
        p.finalized = true;
        let ceiling = p.mint_ceiling;
        let breaker = p.circuit_breaker_engaged;
        cput::set_ceiling_internal(protocol, ceiling, breaker);
        event::emit(Enacted { id, mint_ceiling: ceiling });
    }

    public fun tally(gov: &GovernanceState, id: u64): (u64, u64) {
        assert!(table::contains(&gov.proposals, id), E_NO_PROPOSAL);
        let p = table::borrow(&gov.proposals, id);
        (p.for_power, p.against_power)
    }
}
