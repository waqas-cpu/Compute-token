/// # minting — on-chain Gate 2→4 (agent mint instruction)
module cput::minting {
    use sui::table::{Self, Table};
    use sui::event;
    use cput::cput::{Self, AdminCap, ProtocolState};
    use cput::oracle_verifier::{Self, OracleState};
    use cput::agent_quorum::{Self, AgentQuorumBook};
    use cput::distribution;
    use cput::policy;

    const E_NO_REPORT: u64 = 1;
    const E_EXCEEDS_CEILING: u64 = 2;
    const E_ALREADY_MINTED: u64 = 3;
    const E_NO_AGENT_APPROVAL: u64 = 4;
    const E_AGENT_TOTAL_MISMATCH: u64 = 5;

    public struct MintLog has key {
        id: UID,
        minted: Table<u64, u64>,
    }

    public struct EpochMinted has copy, drop {
        epoch: u64,
        total: u64,
        providers: u64,
        oracle: u64,
        treasury: u64,
        burn_reserve: u64,
        verified_gflops: u64,
    }

    /// Initialise the per-epoch mint replay log.
    public entry fun initialize(_admin: &AdminCap, ctx: &mut TxContext) {
        let log = MintLog {
            id: object::new(ctx),
            minted: table::new(ctx),
        };
        transfer::share_object(log);
    }

    /// Execute an agent mint instruction for `epoch` (Gate 2→4).
    /// Requires prior [`agent_quorum::approve_epoch_mint`] for the same epoch.
    public entry fun execute_mint(
        _admin: &AdminCap,
        protocol: &mut ProtocolState,
        oracle: &OracleState,
        agents: &AgentQuorumBook,
        log: &mut MintLog,
        epoch: u64,
        total: u64,
        ctx: &mut TxContext,
    ) {
        cput::assert_pools_configured(protocol);
        assert!(oracle_verifier::has_report(oracle, epoch), E_NO_REPORT);
        assert!(agent_quorum::has_approved(agents, epoch), E_NO_AGENT_APPROVAL);
        assert!(agent_quorum::approved_total(agents, epoch) == total, E_AGENT_TOTAL_MISMATCH);
        assert!(total <= cput::mint_ceiling(protocol), E_EXCEEDS_CEILING);
        assert!(!table::contains(&log.minted, epoch), E_ALREADY_MINTED);

        let verified_gflops = oracle_verifier::verified_gflops(oracle, epoch);
        table::add(&mut log.minted, epoch, total);

        let split = distribution::distribute(protocol, total, ctx);
        event::emit(EpochMinted {
            epoch,
            total,
            providers: policy::providers(&split),
            oracle: policy::oracle(&split),
            treasury: policy::treasury(&split),
            burn_reserve: policy::burn_reserve(&split),
            verified_gflops,
        });
    }

    public fun is_minted(log: &MintLog, epoch: u64): bool {
        table::contains(&log.minted, epoch)
    }

    public fun recommend_ceiling(current: u64, twap_util_bps: u64): u64 {
        policy::recommend_ceiling(current, twap_util_bps)
    }
}
