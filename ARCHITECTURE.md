# CPUT Architecture

This document specifies the decomposition principles, the per-layer invariants,
and the integration-gate contracts that the prototype implements. It is the
normative companion to the code: every rule below maps to a concrete check in a
named module.

---

## 1. Decomposition model

The system is decomposed along three orthogonal axes.

### 1.1 Vertical decomposition (the layer stack)

Each layer is a bounded crate that consumes the artifact of the layer below
(through a gate), enforces its own invariants, and produces a single typed
artifact for the layer above.

| Layer | Crate | Responsibility | Output artifact |
|-------|-------|----------------|-----------------|
| L0 | `cput-layer0-compute`    | node identity, TEE attestation        | `AttestationPacket` |
| L1 | `cput-layer1-oracle`     | DON verification, proof aggregation   | `EpochReport` |
| L2 | `cput-layer2-agentic`    | multi-agent minting consensus         | `MintInstruction` |
| L4 | `cput-layer4-tokenomics` | distribution, burn market, governor   | `SettlementReceipt` |
| L5 | `cput-layer5-settlement` | settlement, DAO governance, audit     | `PolicyUpdate` (downward) |

> **Why no L3?** The architecture defines the **post-quantum cryptography layer
> as a pervasive horizontal fabric**, not a discrete vertical stage. It is the
> `cput-pqc` crate, used by every layer and every gate. Keeping the L0/L1/L2/L4/L5
> numbering preserves the reference document's labels.

### 1.2 Horizontal decomposition (cross-cutting fabrics)

| Fabric | Crate | Provides |
|--------|-------|----------|
| PQC          | `cput-pqc`     | ML-KEM-1024 (FIPS 203), ML-DSA-87 (FIPS 204), SLH-DSA (FIPS 205), algorithm-tagged sealed envelopes, crypto-agility registry |
| ZK           | `cput-zk`      | `ProofBackend` trait, statement encoders, deterministic reference backend |
| Audit        | `cput-audit`   | append-only Merkle log, root computation |
| Message-auth | `cput-msgauth` | authenticated messages (ML-DSA MAC) + monotonic anti-replay guard |

### 1.3 Integration gates

A gate is the **only** sanctioned path across a layer boundary. Every gate:

1. deserialises a single typed contract,
2. verifies the producer's PQC signature against the approved-algorithm
   registry (crypto-agility),
3. verifies any attached ZK proof against its public-input commitment,
4. checks the boundary's economic / structural invariants,

and returns a typed error on any failure. Gates are implemented in `cput-gates`
(off-chain) and mirrored in the Move modules (on-chain).

---

## 2. Per-layer invariants

### L0 — compute (`cput-layer0-compute`)
- **R0.1 Identity binding.** A `NodeId` is the SHA-256 of the node's committed
  ML-DSA-87 public key.
- **R0.2 Capacity bound.** Attested GFLOPs ≤ rated capacity for the epoch
  duration (anti-overclaim).
- **R0.3 Sealed attestation.** Every attestation body is ML-DSA-signed and
  carries a ZK compute proof over the same public inputs.
- **R0.4 Telemetry sanity.** Bounded thermal envelope; zero-GFLOP attestations
  are never produced.

### L1 — oracle (`cput-layer1-oracle`)
- **R1.1 Re-verification.** Every attestation is re-checked at Gate 0→1; the
  oracle never trusts a node's self-report.
- **R1.2 Quorum.** A report is emitted only with ≥ `ORACLE_THRESHOLD` of
  `ORACLE_SET_SIZE` operator signatures over the identical canonical body.
- **R1.3 Proof folding.** The aggregate proof folds exactly one leaf proof per
  scored node; the body commits to the aggregate.
- **R1.4 Reconciliation.** Per-node GFLOP scores sum to the reported total.
- **R1.5 Dispute window.** The dispute window closes `DISPUTE_WINDOW_EPOCHS`
  after the reported epoch.

### L2 — agentic minting (`cput-layer2-agentic`)
- **R2.1 Verified input only.** The epoch report is re-admitted at Gate 1→2
  before any agent reasons over it.
- **R2.2 Independent quorum.** `AGENT_QUORUM_SIZE` agents each produce a
  proposal; the spread must stay within `CONSENSUS_TOLERANCE_BPS` (±2%).
- **R2.3 Conservation.** Per-node allocations sum to the consensus total.
- **R2.4 Committed policy.** The emission policy is hashed and bound into the
  instruction and the reasoning proof.

### L4 — tokenomics (`cput-layer4-tokenomics`)
- **R4.1 Ceiling enforcement.** No mint exceeds the live per-epoch ceiling.
- **R4.2 Conservative split.** providers/oracle/treasury/burn shares sum to the
  minted total exactly (rounding remainder folded into providers).
- **R4.3 Auditability.** Every mint/burn is appended to the audit log before the
  receipt is signed; the receipt commits to the resulting root.
- **R4.4 Burn market.** Access fees split burn/provider/treasury per policy; the
  burned portion reduces circulating supply.
- **R4.5 Elastic ceiling.** The governor moves the ceiling within bounded steps
  from a smoothed (TWAP) utilisation signal.

### L5 — settlement & governance (`cput-layer5-settlement`)
- **R5.1 Settlement finality.** A receipt is finalised only after clearing
  Gate 4→5.
- **R5.2 Signed audit root.** Each settled epoch's audit root is committed under
  an SLH-DSA signature.
- **R5.3 Quadratic voting.** Voting power is the integer square root of token
  weight.
- **R5.4 Safe policy.** A zero mint ceiling is only emitted with the circuit
  breaker engaged.

---

## 3. Gate contracts

| Gate | Contract type | Producer rule | Consumer checks |
|------|---------------|---------------|-----------------|
| 0→1   | `AttestationPacket` | L0 seals body w/ ML-DSA + ZK proof | sig valid, proof binds to body, capacity bound |
| 1→2   | `EpochReport`       | L1 collects t-of-n sigs + aggregate proof | threshold met, distinct signers, reconciliation, aggregate commitment |
| 2→4   | `MintInstruction`   | L2 coordinator seals body + reasoning proof | sig valid, proof binds, `total ≤ ceiling`, allocations sum to total |
| 4→5   | `SettlementReceipt` | L4 engine seals split | sig valid, split sums to minted total, shares match policy formula |
| 5→2/4 | `PolicyUpdate`      | L5 DAO seals policy w/ SLH-DSA | sig valid, safe-ceiling rule, approved-algorithm set well-formed |

---

## 4. Policy constants (single source of truth)

Defined in `cput-core::policy` and mirrored in the Move modules.

| Constant | Value | Meaning |
|----------|-------|---------|
| `EPOCH_SECONDS` | 360 | epoch length (6 min) |
| `ORACLE_SET_SIZE` / `ORACLE_THRESHOLD` | 9 / 5 | DON size / quorum |
| `AGENT_QUORUM_SIZE` | 3 | minting agents |
| `CONSENSUS_TOLERANCE_BPS` | 200 | ±2% agent agreement |
| `SPLIT_*` | 7000 / 1500 / 1000 / 500 | providers / oracle / treasury / burn |
| `BURN_FEE_*` | 2000 / 7500 / 500 | burned / providers / treasury |
| `UTIL_LOW_BPS` / `UTIL_HIGH_BPS` | 6000 / 9000 | governor thresholds |
| `CEILING_DECREASE_BPS` / `CEILING_INCREASE_BPS` | 500 / 300 | governor step caps |
| `TWAP_WINDOW_EPOCHS` | 1680 | utilisation smoothing (~7 days) |
| `DISPUTE_WINDOW_EPOCHS` | 7 | dispute window |

The split tables are validated at the type level: `distribution_split_is_valid()`
and `burn_market_split_is_valid()` both assert their shares sum to 100%.

---

## 5. On-chain mirror (Move / Sui) — deploy target

| Module | Mirrors | Enforces on-chain |
|--------|---------|-------------------|
| `cput`            | Sui Coin + protocol state | fungible `Coin<CPUT>`, pools, ceiling, mint authority |
| `policy`          | `cput_core::policy`       | shared economic constants |
| `oracle_verifier` | Gate 1→2                  | t-of-n threshold + distinct signers |
| `minting`         | Gate 2→4                  | report exists, ceiling, replay protection |
| `distribution`    | R4.2                      | split sums to total, mints to pools |
| `burn`            | R4.4                      | utility/payment fee routing |
| `governance`      | R5.3 / Gate 5→2/4         | quadratic tally, enacts ceiling |
| `staking`         | staking/slashing          | escrowed stake, slash-burns supply |
| `audit_registry`  | `cput-audit` / R5.2       | on-chain audit-root anchor per epoch |

**Off-chain relayer (`cput-sui`):** verifies PQC/ZK off-chain, posts
`submit_report` / `execute_mint` / `set_ceiling`, reconciles `EpochMinted`
events against `SettlementReceipt` via JSON-RPC.

Legacy Aptos modules remain in `move/` for reference only.
