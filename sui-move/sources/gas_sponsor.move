/// # gas_sponsor — on-chain gas sponsorship registry (account abstraction prep)
///
/// Tracks which off-chain relayer addresses may submit protocol transactions
/// with gas paid by a sponsor wallet. Actual gas payment is performed at the
/// transaction layer (`--gas-owner` in the relayer); this module records
/// authorization and sponsored-epoch audit trail.
module cput::gas_sponsor {
    use sui::event;
    use sui::table::{Self, Table};
    use cput::cput::AdminCap;

    /// Owned capability held by the gas sponsor operator (DAO / gas station).
    public struct SponsorCap has key, store {
        id: UID,
    }

    /// Shared registry of authorized relayers and sponsored epochs.
    public struct GasSponsorState has key {
        id: UID,
        sponsor_admin: address,
        authorized_senders: vector<address>,
        epochs_sponsored: Table<u64, bool>,
        total_sponsored: u64,
    }

    const E_NOT_AUTHORIZED: u64 = 1;
    const E_ALREADY_AUTHORIZED: u64 = 2;
    const E_NOT_IN_LIST: u64 = 3;
    const E_DUPLICATE_EPOCH: u64 = 4;

    public struct GasSponsorInitialized has copy, drop {
        sponsor_admin: address,
    }

    public struct SenderAuthorized has copy, drop {
        sender: address,
    }

    public struct EpochSponsored has copy, drop {
        epoch: u64,
        sender: address,
    }

    /// Initialize the gas sponsor registry (called once after publish).
    public entry fun initialize(_admin: &AdminCap, ctx: &mut TxContext) {
        let admin = ctx.sender();
        let cap = SponsorCap { id: object::new(ctx) };
        let state = GasSponsorState {
            id: object::new(ctx),
            sponsor_admin: admin,
            authorized_senders: vector[],
            epochs_sponsored: table::new(ctx),
            total_sponsored: 0,
        };
        transfer::transfer(cap, admin);
        transfer::share_object(state);
        event::emit(GasSponsorInitialized { sponsor_admin: admin });
    }

    /// Authorize a relayer address to submit sponsored protocol transactions.
    public entry fun authorize_sender(
        _cap: &SponsorCap,
        state: &mut GasSponsorState,
        sender: address,
    ) {
        assert!(!vector::contains(&state.authorized_senders, &sender), E_ALREADY_AUTHORIZED);
        vector::push_back(&mut state.authorized_senders, sender);
        event::emit(SenderAuthorized { sender });
    }

    /// Revoke a previously authorized relayer.
    public entry fun revoke_sender(
        _cap: &SponsorCap,
        state: &mut GasSponsorState,
        sender: address,
    ) {
        let (found, idx) = vector::index_of(&state.authorized_senders, &sender);
        assert!(found, E_NOT_IN_LIST);
        vector::remove(&mut state.authorized_senders, idx);
    }

    /// Record that `sender` sponsored gas for `epoch` (audit / rate-limit hook).
    public entry fun record_sponsored_epoch(
        state: &mut GasSponsorState,
        epoch: u64,
        ctx: &TxContext,
    ) {
        let sender = ctx.sender();
        assert!(is_authorized(state, sender), E_NOT_AUTHORIZED);
        assert!(!table::contains(&state.epochs_sponsored, epoch), E_DUPLICATE_EPOCH);
        table::add(&mut state.epochs_sponsored, epoch, true);
        state.total_sponsored = state.total_sponsored + 1;
        event::emit(EpochSponsored { epoch, sender });
    }

    public fun is_authorized(state: &GasSponsorState, sender: address): bool {
        vector::contains(&state.authorized_senders, &sender)
    }

    public fun total_sponsored(state: &GasSponsorState): u64 {
        state.total_sponsored
    }
}
