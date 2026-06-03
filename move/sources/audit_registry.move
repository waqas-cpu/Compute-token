/// # audit_registry — append-only Merkle audit log (HORIZONTAL fabric)
///
/// On-chain mirror of `cput-audit`. Layer 5 commits one SLH-DSA-signed Merkle
/// root per settled epoch: *"every mint, burn, slash, governance action and
/// bridge transfer is committed to an append-only tree; the root is signed with
/// SLH-DSA at each epoch."* This module stores the monotonic sequence of signed
/// roots so any party can audit historical state transitions.
module cput::audit_registry {
    use std::signer;
    use std::vector;
    use aptos_framework::event;

    /// Caller is not the protocol admin.
    const E_NOT_ADMIN: u64 = 1;
    /// Registry not initialised.
    const E_NOT_INIT: u64 = 2;
    /// Epoch roots must be committed in strictly increasing epoch order.
    const E_NON_MONOTONIC: u64 = 3;

    /// A single committed epoch root.
    struct EpochRoot has store, drop, copy {
        epoch: u64,
        size: u64,
        root: vector<u8>,
        /// SLH-DSA signature bytes over (epoch, size, root). Verified off-chain
        /// / by a future native; stored here as the long-lived commitment.
        signature: vector<u8>,
    }

    /// The append-only log of epoch roots.
    struct AuditLog has key {
        roots: vector<EpochRoot>,
        last_epoch: u64,
        initialized: bool,
    }

    #[event]
    struct RootCommitted has drop, store { epoch: u64, size: u64, root: vector<u8> }

    /// Initialise an empty audit log under the admin account.
    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        move_to(admin, AuditLog { roots: vector::empty<EpochRoot>(), last_epoch: 0, initialized: true });
    }

    /// Commit a signed root for `epoch`. Enforces strictly-increasing epochs
    /// (append-only, no rewrites).
    public fun commit_root(
        epoch: u64,
        size: u64,
        root: vector<u8>,
        signature: vector<u8>,
    ) acquires AuditLog {
        assert!(exists<AuditLog>(@cput), E_NOT_INIT);
        let log = borrow_global_mut<AuditLog>(@cput);
        if (vector::length(&log.roots) > 0) {
            assert!(epoch > log.last_epoch, E_NON_MONOTONIC);
        };
        vector::push_back(&mut log.roots, EpochRoot { epoch, size, root, signature });
        log.last_epoch = epoch;
        event::emit(RootCommitted { epoch, size, root });
    }

    // Number of committed epoch roots.
    #[view]
    public fun len(): u64 acquires AuditLog {
        vector::length(&borrow_global<AuditLog>(@cput).roots)
    }

    // The most recently committed epoch number.
    #[view]
    public fun last_epoch(): u64 acquires AuditLog {
        borrow_global<AuditLog>(@cput).last_epoch
    }
}
