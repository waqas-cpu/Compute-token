/// # compliance — ERC-1400 compliance registry (transfer restrictions / KYC)
///
/// Implements the security-token compliance rules: each holder has a KYC status
/// and a jurisdiction tag, and transfers are gated by a partition policy. This
/// mirrors ERC-1400's "can the transfer happen?" check that the token module
/// consults before moving balances.
module cput::compliance {
    use std::signer;
    use aptos_framework::table::{Self, Table};

    /// Caller is not the protocol admin.
    const E_NOT_ADMIN: u64 = 1;
    /// Holder is not KYC-verified.
    const E_NOT_KYC: u64 = 2;
    /// Holder record does not exist.
    const E_NO_HOLDER: u64 = 3;
    /// Transfer violates a partition / jurisdiction restriction.
    const E_RESTRICTED: u64 = 4;

    /// Standard ERC-1400 partitions.
    const PARTITION_UNRESTRICTED: u8 = 0;
    /// Locked partition (e.g. team/investor vesting) — non-transferable.
    const PARTITION_LOCKED: u8 = 1;
    /// Regulated partition — only transferable between same-jurisdiction KYC holders.
    const PARTITION_REGULATED: u8 = 2;

    /// Per-holder compliance record.
    struct Holder has store, drop, copy {
        kyc_verified: bool,
        jurisdiction: u16,
        partition: u8,
    }

    /// The compliance registry.
    struct Registry has key {
        holders: Table<address, Holder>,
    }

    /// Initialise the compliance registry.
    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        move_to(admin, Registry { holders: table::new<address, Holder>() });
    }

    /// Register or update a holder's compliance record (admin only).
    public entry fun upsert_holder(
        admin: &signer,
        holder: address,
        kyc_verified: bool,
        jurisdiction: u16,
        partition: u8,
    ) acquires Registry {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        let reg = borrow_global_mut<Registry>(@cput);
        let record = Holder { kyc_verified, jurisdiction, partition };
        table::upsert(&mut reg.holders, holder, record);
    }

    /// Abort unless `holder` is KYC-verified.
    public fun assert_kyc(holder: address) acquires Registry {
        let reg = borrow_global<Registry>(@cput);
        assert!(table::contains(&reg.holders, holder), E_NO_HOLDER);
        assert!(table::borrow(&reg.holders, holder).kyc_verified, E_NOT_KYC);
    }

    /// ERC-1400 `canTransfer`: both parties KYC-verified and the partition
    /// policy permits the move. Aborts with the reason code otherwise.
    public fun assert_can_transfer(from: address, to: address) acquires Registry {
        let reg = borrow_global<Registry>(@cput);
        assert!(table::contains(&reg.holders, from), E_NO_HOLDER);
        assert!(table::contains(&reg.holders, to), E_NO_HOLDER);
        let f = table::borrow(&reg.holders, from);
        let t = table::borrow(&reg.holders, to);
        assert!(f.kyc_verified && t.kyc_verified, E_NOT_KYC);
        // Locked partition cannot transfer.
        assert!(f.partition != PARTITION_LOCKED, E_RESTRICTED);
        // Regulated partition: same jurisdiction only.
        if (f.partition == PARTITION_REGULATED) {
            assert!(f.jurisdiction == t.jurisdiction, E_RESTRICTED);
        };
    }

    #[view]
    public fun is_kyc(holder: address): bool acquires Registry {
        let reg = borrow_global<Registry>(@cput);
        table::contains(&reg.holders, holder) && table::borrow(&reg.holders, holder).kyc_verified
    }
}
