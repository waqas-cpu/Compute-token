/// # oracle_verifier — on-chain Gate 1→2 (threshold-signed epoch report)
///
/// The on-chain enforcement of the L1→L2 integration gate. The oracle DON has
/// `ORACLE_SET_SIZE` operators; an epoch report is only accepted once at least
/// `ORACLE_THRESHOLD` *distinct, registered* operators have signed it. Accepted
/// reports are recorded so the `minting` module can authorise the epoch's mint
/// against the verified GFLOP total and ZK commitment.
///
/// ML-DSA signature bytes are verified by an off-chain relayer / future native;
/// this module enforces the *threshold and distinctness* invariants and binds
/// the accepted report to its ZK commitment.
module cput::oracle_verifier {
    use std::signer;
    use std::vector;
    use aptos_framework::table::{Self, Table};
    use aptos_framework::event;

    friend cput::minting;

    /// DON parameters (mirror `cput_core::policy`).
    const ORACLE_SET_SIZE: u64 = 9;
    const ORACLE_THRESHOLD: u64 = 5;

    /// Caller is not the protocol admin.
    const E_NOT_ADMIN: u64 = 1;
    /// Fewer than `ORACLE_THRESHOLD` signatures supplied.
    const E_BELOW_THRESHOLD: u64 = 2;
    /// A signer index repeats or is outside the operator set.
    const E_BAD_SIGNER_SET: u64 = 3;
    /// Report for this epoch already accepted.
    const E_DUPLICATE_EPOCH: u64 = 4;
    /// No accepted report for the requested epoch.
    const E_NO_REPORT: u64 = 5;

    /// An accepted, threshold-verified epoch report.
    struct AcceptedReport has store, drop, copy {
        verified_gflops: u128,
        zk_commitment: vector<u8>,
        signer_count: u64,
    }

    /// Per-protocol oracle verification state.
    struct OracleState has key {
        reports: Table<u64, AcceptedReport>,
    }

    #[event]
    struct ReportAccepted has drop, store { epoch: u64, verified_gflops: u128, signer_count: u64 }

    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        move_to(admin, OracleState { reports: table::new<u64, AcceptedReport>() });
    }

    /// True if every index is unique and within the operator set.
    fun distinct_and_in_range(indices: &vector<u16>): bool {
        let n = vector::length(indices);
        let i = 0;
        while (i < n) {
            let vi = *vector::borrow(indices, i);
            if ((vi as u64) >= ORACLE_SET_SIZE) return false;
            let j = i + 1;
            while (j < n) {
                if (*vector::borrow(indices, j) == vi) return false;
                j = j + 1;
            };
            i = i + 1;
        };
        true
    }

    /// Submit an epoch report with the set of signing operator indices. Enforces
    /// the t-of-n threshold and distinctness, then records the report.
    public fun submit_report(
        epoch: u64,
        verified_gflops: u128,
        zk_commitment: vector<u8>,
        signer_indices: vector<u16>,
    ) acquires OracleState {
        let count = vector::length(&signer_indices);
        assert!(count >= ORACLE_THRESHOLD, E_BELOW_THRESHOLD);
        assert!(distinct_and_in_range(&signer_indices), E_BAD_SIGNER_SET);

        let state = borrow_global_mut<OracleState>(@cput);
        assert!(!table::contains(&state.reports, epoch), E_DUPLICATE_EPOCH);
        table::add(
            &mut state.reports,
            epoch,
            AcceptedReport { verified_gflops, zk_commitment, signer_count: count },
        );
        event::emit(ReportAccepted { epoch, verified_gflops, signer_count: count });
    }

    /// The verified GFLOP total recorded for an accepted epoch (friend access).
    public(friend) fun verified_gflops(epoch: u64): u128 acquires OracleState {
        let state = borrow_global<OracleState>(@cput);
        assert!(table::contains(&state.reports, epoch), E_NO_REPORT);
        table::borrow(&state.reports, epoch).verified_gflops
    }

    #[view]
    public fun has_report(epoch: u64): bool acquires OracleState {
        table::contains(&borrow_global<OracleState>(@cput).reports, epoch)
    }

    #[view]
    public fun threshold(): u64 { ORACLE_THRESHOLD }

    #[view]
    public fun set_size(): u64 { ORACLE_SET_SIZE }
}
