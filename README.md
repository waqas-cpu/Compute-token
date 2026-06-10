# CPUT — Compute Utility Token

A complete, compilable prototype of the **Compute Utility Token (CPUT)** protocol:
a post-quantum-secured, ZK-verified network that mints tokens against
independently-verified GPU/TPU compute.

The system is built strictly around the architecture's three structuring
principles:

- **Vertical decomposition** — six bounded layers (L0–L5), each a Rust crate
  with explicit, enforced invariants.
- **Horizontal decomposition** — four cross-cutting fabrics (PQC, ZK, audit,
  message-auth) that every layer shares.
- **Integration gates** — five typed, validated boundary contracts; data may
  only cross a layer boundary through a gate that re-checks signatures, ZK
  proofs, replay, and economic invariants.

Off-chain logic is **Rust**; on-chain smart contracts are **Move** (Aptos prototype
in `move/`, **Sui deployment package** in `sui-move/`).

> This is a prototype. The reference ZK backend is deterministic and **not**
> zero-knowledge, and on-chain PQC verification is abstracted. See
> [`PRODUCTION.md`](./PRODUCTION.md) for the path to a production deployment.

---

## Repository layout

```
crates/
  cput-core/              shared domain model, units, IDs, errors, policy constants
  cput-pqc/               HORIZONTAL: ML-KEM-1024, ML-DSA-87, SLH-DSA + crypto-agility registry
  cput-zk/                HORIZONTAL: proof backend trait + deterministic reference backend
  cput-audit/             HORIZONTAL: append-only Merkle audit log
  cput-msgauth/           HORIZONTAL: authenticated messaging + anti-replay
  cput-gates/             the 5 typed integration-gate contracts + validators
  cput-layer0-compute/    VERTICAL L0: node identity, TEE attestation
  cput-layer1-oracle/     VERTICAL L1: oracle DON, proof aggregation, epoch report
  cput-layer2-agentic/    VERTICAL L2: multi-agent minting consensus
  cput-layer4-tokenomics/ VERTICAL L4: distribution split, burn market, elastic governor
  cput-layer5-settlement/ VERTICAL L5: settlement, DAO governance, audit commitment
  cput-store/             durable relayer / epoch persistence (sled)
  cput-sui/               Sui adapters, RPC reconcile, relayer (production bridge)
  bins/
    cput-node/            end-to-end pipeline demo (L0→L1→L2→L4→L5 + downward gate)
    cput-oracle/          oracle DON daemon (Layer 1) + Sui submit intent
    cput-agent/           minting agent daemon (Layer 2) + Sui mint intent
    cput-relayer/         production Sui relayer (post + reconcile)
move/
  sources/                on-chain Move package (11 modules, Aptos)
sui-move/
  sources/                Sui Coin-standard CPUT package (9 modules)
ARCHITECTURE.md           decomposition rules, gate specs, invariants
PRODUCTION.md             production-readiness checklist & integration points
```

> Layer 3 is intentionally absent from the vertical stack: the architecture
> defines **PQC as a pervasive horizontal security fabric**, not a discrete
> layer. It lives in `cput-pqc` and underpins every gate.

---

## Build & run

Prerequisites: Rust (pinned by `rust-toolchain.toml`) and the
[Aptos CLI](https://aptos.dev/tools/aptos-cli/).

### Off-chain (Rust)

```bash
cargo build --workspace          # compile everything
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

# run the end-to-end pipeline demonstration
cargo run --bin cput-node
# focused single-layer demos (emit Sui transaction intents)
cargo run --bin cput-oracle
cargo run --bin cput-agent
# production relayer (dry-run by default — see config/cput.example.toml)
cargo run --bin cput-relayer -- --dry-run
```

### On-chain (Move / Aptos)

```bash
cd move
aptos move compile             # compiles all 11 modules
```

### On-chain (Move / Sui) — **deploy target**

Prerequisites: **Sui CLI via [suiup](https://github.com/MystenLabs/suiup)** (testnet
toolchain) and a local checkout of the Sui framework at `.sui-framework/` (used by
`sui-move/Move.toml`).

```powershell
# One-time machine setup (Windows)
.\scripts\setup-sui-dev.ps1
# Fund address at https://faucet.sui.io then:
.\scripts\after-faucet-deploy.ps1
```

```bash
cd sui-move
sui move build                 # compile the CPUT package
sui move test                  # economic invariant unit tests

# publish to testnet (after `sui client` is configured)
sui client publish --gas-budget 200000000
```

**Testnet first** (mainnet blocked until hardening in [`PRODUCTION.md`](./PRODUCTION.md)):

```powershell
# Fund deployer: https://faucet.sui.io (CLI v1.50 redirects to web faucet)
.\scripts\deploy-testnet.ps1 -SuiCli ".\.sui-bin\sui.exe"
```

Post-publish setup (with `AdminCap`):

1. `oracle_verifier::initialize`
2. `minting::initialize`
3. `governance::initialize`
4. `staking::initialize`
5. `audit_registry::initialize`
6. `cput::configure_pools` — provider / oracle / treasury / burn-reserve addresses
7. `cput::set_ceiling` — genesis per-epoch mint cap

Per epoch: `oracle_verifier::submit_report` → `minting::execute_mint`.
For payment/utility fees: `burn::pay_access_fee` with a `Coin<CPUT>`.

### Agent quorum health (Layer 2)

Three **formula agents** (`MintingAgent` 0–2) run the committed emission policy.
Before relaying to Sui:

```bash
cargo run --release --bin cput-agent -- --health   # JSON readiness report, exit 0 if healthy
cargo run --release --bin cput-relayer -- --config config/cput.toml --dry-run
```

The relayer aborts if quorum spread exceeds `CONSENSUS_TOLERANCE_BPS` (200 bps).

---

## The five integration gates

| Gate | Producer → Consumer | Payload | Core checks |
|------|--------------------|---------|-------------|
| 0→1  | L0 → L1 | `AttestationPacket`  | ML-DSA sig, ZK compute proof, capacity bound |
| 1→2  | L1 → L2 | `EpochReport`        | t-of-n threshold sigs, aggregate proof, reconciliation |
| 2→4  | L2 → L4 | `MintInstruction`    | agent sig, reasoning proof, ceiling, conservation |
| 4→5  | L4 → L5 | `SettlementReceipt`  | engine sig, distribution split sums to total |
| 5→2/4| L5 → L2/L4 | `PolicyUpdate`    | SLH-DSA gov sig, safe-ceiling rule |

Each gate is implemented twice: once off-chain (`cput-gates`) and once on-chain
(the Move modules), so the boundary rules are enforced on both sides.

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for the full rule set and per-layer
invariants.
