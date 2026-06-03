/// # governance — DAO governance (quadratic voting) + downward Gate 5→2/4
///
/// The DAO that retunes minting parameters. Voting power is the integer square
/// root of a voter's token balance (quadratic voting, mitigating plutocracy).
/// A passed proposal is enacted as a policy update that flows back down through
/// the governor (the downward gate 5→2/4), optionally publishing newly approved
/// PQC algorithms (crypto-agility). The DAO signs enacted policy with SLH-DSA
/// off-chain; this module enforces the tally and safety invariants on-chain.
module cput::governance {
    use std::signer;
    use std::vector;
    use aptos_framework::table::{Self, Table};
    use aptos_framework::event;
    use cput::cput_token;
    use cput::governor;

    /// Caller is not the protocol admin.
    const E_NOT_ADMIN: u64 = 1;
    /// Proposal does not exist.
    const E_NO_PROPOSAL: u64 = 2;
    /// Voter has already voted on this proposal.
    const E_DOUBLE_VOTE: u64 = 3;
    /// Proposal already finalised.
    const E_FINALIZED: u64 = 4;
    /// Proposal did not pass.
    const E_NOT_PASSED: u64 = 5;
    /// Zero ceiling requires the circuit breaker (R5.4).
    const E_UNSAFE_POLICY: u64 = 6;

    struct Proposal has store {
        effective_epoch: u64,
        mint_ceiling: u128,
        circuit_breaker_engaged: bool,
        for_power: u128,
        against_power: u128,
        voters: vector<address>,
        finalized: bool,
    }

    struct Governance has key {
        proposals: Table<u64, Proposal>,
        next_id: u64,
    }

    #[event]
    struct ProposalCreated has drop, store { id: u64, mint_ceiling: u128 }
    #[event]
    struct Voted has drop, store { id: u64, voter: address, power: u128, support: bool }
    #[event]
    struct Enacted has drop, store { id: u64, mint_ceiling: u128 }

    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        move_to(admin, Governance { proposals: table::new<u64, Proposal>(), next_id: 0 });
    }

    /// Integer square root (floor) — the quadratic-voting weighting (R5.3).
    public fun isqrt(n: u128): u128 {
        if (n < 2) return n;
        let x = n;
        let y = (x + 1) / 2;
        while (y < x) {
            x = y;
            y = (x + n / x) / 2;
        };
        x
    }

    /// Create a proposal to retune the mint ceiling. Returns its id.
    public entry fun create_proposal(
        admin: &signer,
        effective_epoch: u64,
        mint_ceiling: u128,
        circuit_breaker_engaged: bool,
    ) acquires Governance {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        let gov = borrow_global_mut<Governance>(@cput);
        let id = gov.next_id;
        gov.next_id = id + 1;
        table::add(&mut gov.proposals, id, Proposal {
            effective_epoch,
            mint_ceiling,
            circuit_breaker_engaged,
            for_power: 0,
            against_power: 0,
            voters: vector::empty<address>(),
            finalized: false,
        });
        event::emit(ProposalCreated { id, mint_ceiling });
    }

    /// Cast a quadratic vote weighted by the voter's current token balance.
    public entry fun vote(voter: &signer, id: u64, support: bool) acquires Governance {
        let who = signer::address_of(voter);
        let gov = borrow_global_mut<Governance>(@cput);
        assert!(table::contains(&gov.proposals, id), E_NO_PROPOSAL);
        let p = table::borrow_mut(&mut gov.proposals, id);
        assert!(!p.finalized, E_FINALIZED);
        assert!(!vector::contains(&p.voters, &who), E_DOUBLE_VOTE);

        let power = isqrt(cput_token::balance_of(who));
        if (support) { p.for_power = p.for_power + power; }
        else { p.against_power = p.against_power + power; };
        vector::push_back(&mut p.voters, who);
        event::emit(Voted { id, voter: who, power, support });
    }

    /// Finalise a passed proposal, enacting the new ceiling through the governor
    /// (the downward gate 5→2/4). Enforces R5.4 before enacting.
    public entry fun finalize(admin: &signer, id: u64) acquires Governance {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        let gov = borrow_global_mut<Governance>(@cput);
        assert!(table::contains(&gov.proposals, id), E_NO_PROPOSAL);
        let p = table::borrow_mut(&mut gov.proposals, id);
        assert!(!p.finalized, E_FINALIZED);
        assert!(p.for_power > p.against_power, E_NOT_PASSED);
        assert!(p.mint_ceiling > 0 || p.circuit_breaker_engaged, E_UNSAFE_POLICY);
        p.finalized = true;
        let ceiling = p.mint_ceiling;
        let breaker = p.circuit_breaker_engaged;
        governor::set_ceiling(ceiling, breaker);
        event::emit(Enacted { id, mint_ceiling: ceiling });
    }

    #[view]
    public fun tally(id: u64): (u128, u128) acquires Governance {
        let gov = borrow_global<Governance>(@cput);
        assert!(table::contains(&gov.proposals, id), E_NO_PROPOSAL);
        let p = table::borrow(&gov.proposals, id);
        (p.for_power, p.against_power)
    }
}
