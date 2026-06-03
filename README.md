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

Off-chain logic is **Rust**; on-chain smart contracts are **Move (Aptos)**.

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
  bins/
    cput-node/            end-to-end pipeline demo (L0→L1→L2→L4→L5 + downward gate)
    cput-oracle/          oracle DON daemon demo (Layer 1)
    cput-agent/           minting agent daemon demo (Layer 2)
move/
  sources/                on-chain Move package (11 modules)
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
# focused single-layer demos
cargo run --bin cput-oracle
cargo run --bin cput-agent
```

### On-chain (Move / Aptos)

```bash
cd move
aptos move compile             # compiles all 11 modules
```

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
