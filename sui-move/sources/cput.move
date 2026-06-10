/// # cput — $CPUT fungible utility + payment token (Sui Coin standard)
///
/// Creates a transferable `Coin<CPUT>` using the Sui `coin` module. Minting and
/// burning are restricted to package-internal protocol paths so supply only
/// changes through the audited minting gate and burn market. Holders transfer
/// coins freely as standard Sui fungible assets (wallet / DEX compatible).
module cput::cput {
    use sui::coin::{Self, Coin, TreasuryCap};
    use sui::event;

    /// One-time witness for the CPUT currency type (module name: `cput`).
    public struct CPUT has drop {}

    /// Protocol admin capability — transfer to DAO/multisig after deployment.
    public struct AdminCap has key, store {
        id: UID,
    }

    /// Shared protocol ledger holding the mint authority and economic state.
    public struct ProtocolState has key {
        id: UID,
        treasury_cap: TreasuryCap<CPUT>,
        mint_ceiling: u64,
        circuit_breaker_engaged: bool,
        provider_pool: address,
        oracle_pool: address,
        treasury_pool: address,
        burn_reserve_pool: address,
        pools_configured: bool,
    }

    const DECIMALS: u8 = 8;

    const E_POOLS_NOT_CONFIGURED: u64 = 2;
    const E_POOLS_ALREADY_SET: u64 = 3;
    const E_UNSAFE_CEILING: u64 = 4;

    public struct TokenInitialized has copy, drop {
        decimals: u8,
        admin: address,
    }

    public struct PoolsConfigured has copy, drop {
        provider_pool: address,
        oracle_pool: address,
        treasury_pool: address,
        burn_reserve_pool: address,
    }

    public struct CeilingUpdated has copy, drop {
        mint_ceiling: u64,
        circuit_breaker_engaged: bool,
    }

    /// Package publish hook: create the CPUT currency and share protocol state.
    fun init(witness: CPUT, ctx: &mut TxContext) {
        let (treasury_cap, metadata) = coin::create_currency(
            witness,
            DECIMALS,
            b"CPUT",
            b"Compute Utility Token",
            b"Fungible utility and payment token minted against verified GPU/TPU compute. Transfers follow the Sui Coin standard.",
            std::option::none(),
            ctx,
        );

        transfer::public_freeze_object(metadata);

        let admin = ctx.sender();
        let state = ProtocolState {
            id: object::new(ctx),
            treasury_cap,
            mint_ceiling: 0,
            circuit_breaker_engaged: false,
            provider_pool: @0x0,
            oracle_pool: @0x0,
            treasury_pool: @0x0,
            burn_reserve_pool: @0x0,
            pools_configured: false,
        };

        transfer::share_object(state);
        transfer::transfer(AdminCap { id: object::new(ctx) }, admin);
        event::emit(TokenInitialized { decimals: DECIMALS, admin });
    }

    /// Configure distribution pool addresses (one-time setup).
    public entry fun configure_pools(
        _admin: &AdminCap,
        state: &mut ProtocolState,
        provider_pool: address,
        oracle_pool: address,
        treasury_pool: address,
        burn_reserve_pool: address,
    ) {
        assert!(!state.pools_configured, E_POOLS_ALREADY_SET);
        state.provider_pool = provider_pool;
        state.oracle_pool = oracle_pool;
        state.treasury_pool = treasury_pool;
        state.burn_reserve_pool = burn_reserve_pool;
        state.pools_configured = true;
        event::emit(PoolsConfigured {
            provider_pool,
            oracle_pool,
            treasury_pool,
            burn_reserve_pool,
        });
    }

    /// Set the genesis or governance-enacted mint ceiling.
    public entry fun set_ceiling(
        _admin: &AdminCap,
        state: &mut ProtocolState,
        mint_ceiling: u64,
        circuit_breaker_engaged: bool,
    ) {
        set_ceiling_internal(state, mint_ceiling, circuit_breaker_engaged);
    }

    /// Enact a new ceiling from governance (package-internal).
    public(package) fun set_ceiling_internal(
        state: &mut ProtocolState,
        mint_ceiling: u64,
        circuit_breaker_engaged: bool,
    ) {
        assert!(mint_ceiling > 0 || circuit_breaker_engaged, E_UNSAFE_CEILING);
        state.mint_ceiling = mint_ceiling;
        state.circuit_breaker_engaged = circuit_breaker_engaged;
        event::emit(CeilingUpdated { mint_ceiling, circuit_breaker_engaged });
    }

    /// Mint `amount` base units and transfer to `recipient` (package-internal).
    public(package) fun mint_to(
        state: &mut ProtocolState,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        let coin = coin::mint(&mut state.treasury_cap, amount, ctx);
        transfer::public_transfer(coin, recipient);
    }

    /// Burn a coin, reducing circulating supply (package-internal).
    public(package) fun burn_coin(state: &mut ProtocolState, coin: Coin<CPUT>) {
        coin::burn(&mut state.treasury_cap, coin);
    }

    /// Transfer between protocol accounts without extra gates (staking, routing).
    public(package) fun transfer_coin(coin: Coin<CPUT>, recipient: address) {
        transfer::public_transfer(coin, recipient);
    }

    public(package) fun assert_pools_configured(state: &ProtocolState) {
        assert!(state.pools_configured, E_POOLS_NOT_CONFIGURED);
    }

    public(package) fun mint_ceiling(state: &ProtocolState): u64 { state.mint_ceiling }
    public(package) fun provider_pool(state: &ProtocolState): address { state.provider_pool }
    public(package) fun oracle_pool(state: &ProtocolState): address { state.oracle_pool }
    public(package) fun treasury_pool(state: &ProtocolState): address { state.treasury_pool }
    public(package) fun burn_reserve_pool(state: &ProtocolState): address { state.burn_reserve_pool }

    public fun decimals(): u8 { DECIMALS }

    public fun total_supply(state: &ProtocolState): u64 {
        coin::total_supply(&state.treasury_cap)
    }
}
