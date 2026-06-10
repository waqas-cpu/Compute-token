/// # oracle_verifier — on-chain Gate 1→2 (threshold-signed epoch report)
module cput::oracle_verifier {
    use sui::table::{Self, Table};
    use sui::event;
    use cput::cput::AdminCap;
    use cput::policy;

    const E_BELOW_THRESHOLD: u64 = 1;
    const E_BAD_SIGNER_SET: u64 = 2;
    const E_DUPLICATE_EPOCH: u64 = 3;
    const E_NO_REPORT: u64 = 4;

    public struct AcceptedReport has store, drop, copy {
        verified_gflops: u64,
        zk_commitment: vector<u8>,
        signer_count: u64,
    }

    public struct OracleState has key {
        id: UID,
        reports: Table<u64, AcceptedReport>,
    }

    public struct ReportAccepted has copy, drop {
        epoch: u64,
        verified_gflops: u64,
        signer_count: u64,
    }

    /// Create and share the oracle verification registry.
    public entry fun initialize(_admin: &AdminCap, ctx: &mut TxContext) {
        let state = OracleState {
            id: object::new(ctx),
            reports: table::new(ctx),
        };
        transfer::share_object(state);
    }

    fun distinct_and_in_range(indices: &vector<u16>): bool {
        let n = vector::length(indices);
        let mut i = 0;
        while (i < n) {
            let vi = *vector::borrow(indices, i);
            if ((vi as u64) >= policy::oracle_set_size()) return false;
            let mut j = i + 1;
            while (j < n) {
                if (*vector::borrow(indices, j) == vi) return false;
                j = j + 1;
            };
            i = i + 1;
        };
        true
    }

    /// Submit a threshold-signed epoch report (relayer-gated off-chain sig verify).
    public entry fun submit_report(
        _admin: &AdminCap,
        state: &mut OracleState,
        epoch: u64,
        verified_gflops: u64,
        zk_commitment: vector<u8>,
        signer_indices: vector<u16>,
    ) {
        let count = vector::length(&signer_indices);
        assert!(count >= policy::oracle_threshold(), E_BELOW_THRESHOLD);
        assert!(distinct_and_in_range(&signer_indices), E_BAD_SIGNER_SET);
        assert!(!table::contains(&state.reports, epoch), E_DUPLICATE_EPOCH);
        table::add(
            &mut state.reports,
            epoch,
            AcceptedReport { verified_gflops, zk_commitment, signer_count: count },
        );
        event::emit(ReportAccepted { epoch, verified_gflops, signer_count: count });
    }

    public(package) fun has_report(state: &OracleState, epoch: u64): bool {
        table::contains(&state.reports, epoch)
    }

    public(package) fun verified_gflops(state: &OracleState, epoch: u64): u64 {
        assert!(table::contains(&state.reports, epoch), E_NO_REPORT);
        table::borrow(&state.reports, epoch).verified_gflops
    }

    public fun threshold(): u64 { policy::oracle_threshold() }
    public fun set_size(): u64 { policy::oracle_set_size() }
}
