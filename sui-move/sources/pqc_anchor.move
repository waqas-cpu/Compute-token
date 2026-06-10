/// # pqc_anchor — on-chain PQC signature digest registry
///
/// ML-DSA-87 verification runs off-chain (relayer + gates). This module anchors
/// SHA-256 envelope digests on-chain so auditors can tie mint epochs to verified
/// PQC artifacts without native Move ML-DSA support.
module cput::pqc_anchor {
    use sui::table::{Self, Table};
    use sui::event;
    use cput::cput::AdminCap;

    const E_DUPLICATE_EPOCH: u64 = 1;
    const E_EMPTY_DIGESTS: u64 = 2;
    const E_BAD_DIGEST_LEN: u64 = 3;

    /// Role tags mirrored off-chain (0 = coordinator, 1..3 = agent proposals).
    const ROLE_COORDINATOR: u8 = 0;
    const ROLE_AGENT_BASE: u8 = 1;

    public struct PqcAnchorBook has key {
        id: UID,
        /// epoch -> vector of 32-byte envelope digests (coordinator first, then agents).
        digests: Table<u64, vector<vector<u8>>>,
    }

    public struct PqcAnchored has copy, drop {
        epoch: u64,
        digest_count: u64,
    }

    public entry fun initialize(_admin: &AdminCap, ctx: &mut TxContext) {
        let book = PqcAnchorBook {
            id: object::new(ctx),
            digests: table::new(ctx),
        };
        transfer::share_object(book);
    }

    fun digest_is_32(d: &vector<u8>): bool {
        vector::length(d) == 32
    }

    /// Anchor off-chain-verified PQC envelope digests for an epoch.
    /// `digests[0]` = coordinator mint instruction; `digests[1..]` = agent proposals.
    public entry fun anchor_epoch(
        _admin: &AdminCap,
        book: &mut PqcAnchorBook,
        epoch: u64,
        digests: vector<vector<u8>>,
    ) {
        let n = vector::length(&digests);
        assert!(n > 0, E_EMPTY_DIGESTS);
        assert!(!table::contains(&book.digests, epoch), E_DUPLICATE_EPOCH);
        let mut i = 0;
        while (i < n) {
            let d = vector::borrow(&digests, i);
            assert!(digest_is_32(d), E_BAD_DIGEST_LEN);
            i = i + 1;
        };
        table::add(&mut book.digests, epoch, digests);
        event::emit(PqcAnchored { epoch, digest_count: n });
    }

    public fun has_anchor(book: &PqcAnchorBook, epoch: u64): bool {
        table::contains(&book.digests, epoch)
    }

    public fun digest_count(book: &PqcAnchorBook, epoch: u64): u64 {
        vector::length(table::borrow(&book.digests, epoch))
    }

    public fun role_coordinator(): u8 { ROLE_COORDINATOR }
    public fun role_agent_base(): u8 { ROLE_AGENT_BASE }
}
