/// # audit_registry — on-chain epoch audit-root anchor (R5.2 mirror)
///
/// Commits each settled epoch's audit Merkle root under relayer governance,
/// enabling off-chain `EpochCommitment` reconciliation against chain state.
module cput::audit_registry {
    use sui::table::{Self, Table};
    use sui::event;
    use cput::cput::AdminCap;

    const E_DUPLICATE_EPOCH: u64 = 1;

    public struct AuditRegistry has key {
        id: UID,
        roots: Table<u64, vector<u8>>,
    }

    public struct AuditRootCommitted has copy, drop {
        epoch: u64,
        minted_total: u64,
        root_len: u64,
    }

    public entry fun initialize(_admin: &AdminCap, ctx: &mut TxContext) {
        let reg = AuditRegistry {
            id: object::new(ctx),
            roots: table::new(ctx),
        };
        transfer::share_object(reg);
    }

    /// Anchor an epoch audit root (relayer posts after Gate 4→5 finality).
    public entry fun commit_root(
        _admin: &AdminCap,
        reg: &mut AuditRegistry,
        epoch: u64,
        minted_total: u64,
        audit_root: vector<u8>,
    ) {
        assert!(!table::contains(&reg.roots, epoch), E_DUPLICATE_EPOCH);
        let root_len = vector::length(&audit_root);
        table::add(&mut reg.roots, epoch, audit_root);
        event::emit(AuditRootCommitted { epoch, minted_total, root_len });
    }

    public fun has_root(reg: &AuditRegistry, epoch: u64): bool {
        table::contains(&reg.roots, epoch)
    }
}
