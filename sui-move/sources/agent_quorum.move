/// # agent_quorum — on-chain AI agent minting quorum (Gate 2→4)
///
/// The mint path is **agent-centric**: three independent formula agents must
/// submit aligned proposals within [`policy::consensus_tolerance_bps`], a
/// coordinator seals the consensus total + policy/reasoning hashes, and
/// [`minting::execute_mint`] refuses to mint without agent approval.
///
/// ML-DSA verification runs off-chain; this module enforces economic quorum
/// invariants and anchors agent identity via pubkey hashes.
module cput::agent_quorum {
    use sui::table::{Self, Table};
    use sui::event;
    use cput::cput::AdminCap;
    use cput::oracle_verifier::{Self, OracleState};
    use cput::policy;

    const E_BELOW_QUORUM: u64 = 1;
    const E_BAD_AGENT_SET: u64 = 2;
    const E_DUPLICATE_EPOCH: u64 = 3;
    const E_SPREAD_EXCEEDED: u64 = 4;
    const E_TOTAL_MISMATCH: u64 = 5;
    const E_HASH_LEN: u64 = 6;
    const E_NO_ORACLE_REPORT: u64 = 7;
    const E_UNKNOWN_AGENT: u64 = 8;
    const E_INACTIVE_AGENT: u64 = 9;
    const E_FORMULA_MISMATCH: u64 = 10;

    /// Formula-agent kind (emission-policy executor).
    const KIND_FORMULA_AGENT: u8 = 0;

    /// One agent's independent mint proposal for an epoch.
    public struct AgentProposal has store, drop, copy {
        agent_id: u8,
        proposal: u64,
    }

    /// On-chain record of agent quorum consensus for one epoch.
    public struct EpochAgentApproval has store {
        proposals: vector<AgentProposal>,
        consensus_total: u64,
        spread_bps: u64,
        policy_hash: vector<u8>,
        reasoning_trace_hash: vector<u8>,
    }

    /// Registry of AI minting agents (ML-DSA pubkey hash + kind).
    public struct RegisteredAgent has store, drop, copy {
        pubkey_hash: vector<u8>,
        kind: u8,
        active: bool,
    }

    /// Shared agent identity registry.
    public struct AgentRegistry has key {
        id: UID,
        agents: Table<u8, RegisteredAgent>,
    }

    /// Shared per-epoch agent quorum approvals.
    public struct AgentQuorumBook has key {
        id: UID,
        approvals: Table<u64, EpochAgentApproval>,
    }

    public struct AgentRegistered has copy, drop {
        agent_id: u8,
        kind: u8,
    }

    public struct AgentQuorumApproved has copy, drop {
        epoch: u64,
        consensus_total: u64,
        spread_bps: u64,
        agent_count: u64,
    }

    public entry fun initialize_registry(_admin: &AdminCap, ctx: &mut TxContext) {
        let registry = AgentRegistry {
            id: object::new(ctx),
            agents: table::new(ctx),
        };
        transfer::share_object(registry);
    }

    public entry fun initialize_quorum_book(_admin: &AdminCap, ctx: &mut TxContext) {
        let book = AgentQuorumBook {
            id: object::new(ctx),
            approvals: table::new(ctx),
        };
        transfer::share_object(book);
    }

    fun hash_is_32(h: &vector<u8>): bool {
        vector::length(h) == 32
    }

    fun distinct_agent_ids(ids: &vector<u8>): bool {
        let n = vector::length(ids);
        let mut i = 0;
        while (i < n) {
            let vi = *vector::borrow(ids, i);
            let mut j = i + 1;
            while (j < n) {
                if (*vector::borrow(ids, j) == vi) return false;
                j = j + 1;
            };
            i = i + 1;
        };
        true
    }

    fun min3(a: u64, b: u64, c: u64): u64 {
        let mut m = a;
        if (b < m) { m = b };
        if (c < m) { m = c };
        m
    }

    fun max3(a: u64, b: u64, c: u64): u64 {
        let mut m = a;
        if (b > m) { m = b };
        if (c > m) { m = c };
        m
    }

    /// Register an AI minting agent identity (pubkey hash = SHA-256 of ML-DSA pubkey).
    public entry fun register_agent(
        _admin: &AdminCap,
        registry: &mut AgentRegistry,
        agent_id: u8,
        pubkey_hash: vector<u8>,
        kind: u8,
    ) {
        assert!(hash_is_32(&pubkey_hash), E_HASH_LEN);
        let agent = RegisteredAgent { pubkey_hash, kind, active: true };
        if (table::contains(&registry.agents, agent_id)) {
            table::remove(&mut registry.agents, agent_id);
        };
        table::add(&mut registry.agents, agent_id, agent);
        event::emit(AgentRegistered { agent_id, kind });
    }

    /// Record agent quorum approval for an epoch (Gate 2→4 on-chain mirror).
    ///
    /// `agent_ids` and `proposals` must each have length
    /// [`policy::agent_quorum_size`]. `total` must equal the integer consensus
    /// of the three proposals. `policy_hash` and `reasoning_trace_hash` bind the
    /// committed emission formula and agent reasoning trace (R2.4).
    public entry fun approve_epoch_mint(
        _admin: &AdminCap,
        registry: &AgentRegistry,
        book: &mut AgentQuorumBook,
        oracle: &OracleState,
        epoch: u64,
        agent_ids: vector<u8>,
        proposals: vector<u64>,
        total: u64,
        policy_hash: vector<u8>,
        reasoning_trace_hash: vector<u8>,
    ) {
        assert!(oracle_verifier::has_report(oracle, epoch), E_NO_ORACLE_REPORT);
        assert!(!table::contains(&book.approvals, epoch), E_DUPLICATE_EPOCH);
        assert!(vector::length(&agent_ids) == policy::agent_quorum_size(), E_BELOW_QUORUM);
        assert!(vector::length(&proposals) == policy::agent_quorum_size(), E_BELOW_QUORUM);
        assert!(distinct_agent_ids(&agent_ids), E_BAD_AGENT_SET);
        assert!(hash_is_32(&policy_hash), E_HASH_LEN);
        assert!(hash_is_32(&reasoning_trace_hash), E_HASH_LEN);

        let mut stored: vector<AgentProposal> = vector[];
        let mut i = 0;
        while (i < policy::agent_quorum_size()) {
            let aid = *vector::borrow(&agent_ids, i);
            assert!(table::contains(&registry.agents, aid), E_UNKNOWN_AGENT);
            let reg = table::borrow(&registry.agents, aid);
            assert!(reg.active, E_INACTIVE_AGENT);
            let prop = *vector::borrow(&proposals, i);
            vector::push_back(&mut stored, AgentProposal { agent_id: aid, proposal: prop });
            i = i + 1;
        };

        let p0 = vector::borrow(&stored, 0).proposal;
        let p1 = vector::borrow(&stored, 1).proposal;
        let p2 = vector::borrow(&stored, 2).proposal;
        let min_p = min3(p0, p1, p2);
        let max_p = max3(p0, p1, p2);
        let spread = policy::spread_bps(min_p, max_p);
        assert!(spread <= policy::consensus_tolerance_bps(), E_SPREAD_EXCEEDED);

        let consensus = policy::consensus_total(p0, p1, p2);
        assert!(total == consensus, E_TOTAL_MISMATCH);

        // Formula binding: consensus must match verified GFLOPs × tokens-per-GFLOP.
        let verified = oracle_verifier::verified_gflops(oracle, epoch);
        let expected = verified * policy::default_tokens_per_gflop();
        assert!(total == expected, E_FORMULA_MISMATCH);

        table::add(
            &mut book.approvals,
            epoch,
            EpochAgentApproval {
                proposals: stored,
                consensus_total: total,
                spread_bps: spread,
                policy_hash,
                reasoning_trace_hash,
            },
        );
        event::emit(AgentQuorumApproved {
            epoch,
            consensus_total: total,
            spread_bps: spread,
            agent_count: policy::agent_quorum_size(),
        });
    }

    public(package) fun has_approved(book: &AgentQuorumBook, epoch: u64): bool {
        table::contains(&book.approvals, epoch)
    }

    public(package) fun approved_total(book: &AgentQuorumBook, epoch: u64): u64 {
        table::borrow(&book.approvals, epoch).consensus_total
    }

    public fun spread_for_epoch(book: &AgentQuorumBook, epoch: u64): u64 {
        table::borrow(&book.approvals, epoch).spread_bps
    }

    public fun kind_formula_agent(): u8 { KIND_FORMULA_AGENT }
}
