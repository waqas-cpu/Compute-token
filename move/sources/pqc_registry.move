/// # pqc_registry — crypto-agility registry (HORIZONTAL fabric, on-chain mirror)
///
/// On-chain mirror of the off-chain `cput-pqc` `AlgorithmRegistry`. Every
/// signature and ciphertext in the protocol carries a one-byte algorithm tag;
/// this module records which tags are currently approved. New NIST standards
/// can be approved — and old ones deprecated — by governance, without
/// redeploying verifier logic (the crypto-agility property).
///
/// On-chain PQC signature *verification itself* requires a native function /
/// precompile (ML-DSA / SLH-DSA are not in the Aptos framework today); this
/// registry governs *which* algorithms a verifier must accept. See
/// `PRODUCTION.md`.
module cput::pqc_registry {
    use std::signer;
    use std::vector;
    use aptos_framework::event;

    /// Algorithm tags (mirror of `cput_pqc::AlgorithmId::tag`).
    const TAG_ML_KEM_1024: u8 = 0x03;
    const TAG_ML_DSA_87: u8 = 0x04;
    const TAG_SLH_DSA_256S: u8 = 0x05;

    /// Caller is not the protocol admin.
    const E_NOT_ADMIN: u64 = 1;
    /// Algorithm tag is not currently approved.
    const E_NOT_APPROVED: u64 = 2;
    /// Registry already initialised.
    const E_ALREADY_INIT: u64 = 3;

    /// The set of currently approved algorithm tags.
    struct Registry has key {
        approved: vector<u8>,
    }

    #[event]
    struct AlgorithmApproved has drop, store { tag: u8 }

    #[event]
    struct AlgorithmDeprecated has drop, store { tag: u8 }

    /// Initialise the genesis registry approving all three deployed NIST
    /// standards. Must be called by the package admin (the `cput` account).
    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        assert!(!exists<Registry>(@cput), E_ALREADY_INIT);
        let approved = vector::empty<u8>();
        vector::push_back(&mut approved, TAG_ML_DSA_87);
        vector::push_back(&mut approved, TAG_ML_KEM_1024);
        vector::push_back(&mut approved, TAG_SLH_DSA_256S);
        move_to(admin, Registry { approved });
    }

    // True if `tag` is currently approved.
    #[view]
    public fun is_approved(tag: u8): bool acquires Registry {
        let reg = borrow_global<Registry>(@cput);
        vector::contains(&reg.approved, &tag)
    }

    /// Abort unless `tag` is approved (used by on-chain verifiers).
    public fun require_approved(tag: u8) acquires Registry {
        assert!(is_approved(tag), E_NOT_APPROVED);
    }

    /// Approve a new algorithm (governance action).
    public entry fun approve(admin: &signer, tag: u8) acquires Registry {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        let reg = borrow_global_mut<Registry>(@cput);
        if (!vector::contains(&reg.approved, &tag)) {
            vector::push_back(&mut reg.approved, tag);
            event::emit(AlgorithmApproved { tag });
        }
    }

    /// Deprecate an algorithm with a migration window (governance action).
    public entry fun deprecate(admin: &signer, tag: u8) acquires Registry {
        assert!(signer::address_of(admin) == @cput, E_NOT_ADMIN);
        let reg = borrow_global_mut<Registry>(@cput);
        let (found, idx) = vector::index_of(&reg.approved, &tag);
        assert!(found, E_NOT_APPROVED);
        vector::remove(&mut reg.approved, idx);
        event::emit(AlgorithmDeprecated { tag });
    }
}
