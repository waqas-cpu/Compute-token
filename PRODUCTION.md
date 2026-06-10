# CPUT Production-Readiness Guide

This prototype is **structurally complete and compiles**, but several components
are deliberately reference/abstract implementations. This document is the
checklist to harden it into a production system. It is organised by the same
three axes as the architecture, plus a cross-cutting security section.

> **Status legend:** ✅ production-shaped · ⚠️ prototype stand-in · ❌ not yet present

---

## 1. Cross-cutting fabrics (horizontal)

### 1.1 Post-quantum cryptography (`cput-pqc`)
- ✅ ML-KEM-1024 / ML-DSA-87 / SLH-DSA via the FIPS 203/204/205 crates.
- ✅ Algorithm-tagged `SealedEnvelope` and an approved-algorithm registry give
  **crypto-agility**: new NIST standards can be added by governance without
  changing verifier logic.
- ⚠️ **Key management (`cput-keycustody`).** `KeyBackend::{Ephemeral,File,TeeSealed,Hsm}`
  interface added; only Ephemeral/File (public record) work on testnet. Production
  must bind ML-DSA signing keys to the node's TEE (sealing/quoting), store oracle
  DKG shares in HSMs, and rotate per `DKG_ROTATION_EPOCHS`.
- ❌ **Hybrid signatures.** Consider Ed25519+ML-DSA hybrid envelopes during the
  migration window so the chain remains verifiable by classical verifiers.

### 1.2 Zero-knowledge proofs (`cput-zk`)
- ⚠️ The `ReferenceBackend` is **deterministic SHA-256 commitments — NOT
  zero-knowledge and NOT cryptographically binding**. It exists so the pipeline
  composes and the gate logic can be exercised.
- ❌ **Production backend.** Implement `ProofBackend` over a real proving system
  (e.g. Barretenberg/Noir or Halo2). Two circuits are required:
  1. **Compute proof** (L0): proves the attested GFLOPs match the workload
     execution trace without revealing the workload.
  2. **Reasoning proof** (L2): proves the agent ran the committed emission
     policy on the verified inputs.
- ❌ **Recursive aggregation.** L1's `aggregated_proof` currently folds proof
  *commitments*; production needs a recursive SNARK so one succinct proof
  attests to all per-node proofs.

### 1.3 Audit log (`cput-audit`)
- ✅ Append-only Merkle log with SLH-DSA-signable roots.
- ⚠️ In-memory; production needs durable storage and a public inclusion-proof
  API so any party can verify a historical action.

### 1.4 Message authentication (`cput-msgauth`)
- ✅ ML-DSA MACs + monotonic anti-replay guard.
- ⚠️ The replay guard is in-memory per process; production needs a shared,
  persistent sequence store across a layer's replicas.

---

## 2. Vertical layers

| Layer | Production gaps |
|-------|-----------------|
| L0 compute | ⚠️ `tee::TeeVerifier` trait + `ReferenceTeeVerifier` stub. ❌ Real SGX/SEV-SNP/TDX quote verification. ⚠️ Hardware telemetry source. |
| L1 oracle  | ⚠️ `dkg::ThresholdScheme` trait + `IndependentSigScheme` (current behaviour). ❌ Real DKG ceremony + threshold crypto. ❌ Challenge-sampling & disputes. |
| L2 agentic | ⚠️ Three `MintingAgent` formula agents (`bias_bps=0`); `assess_quorum_health()` + `cput-agent --health`. ❌ Independent inference models; ❌ Byzantine eviction. |
| L4 tokenomics | ⚠️ TWAP utilisation is an input; production must source it from real demand telemetry. ⚠️ State is in-memory; back it with the chain as source of truth. |
| L5 settlement | ⚠️ Governance is single-process; production binds it to the on-chain `governance` module and a real proposal lifecycle (timelocks, quorums). |

---

## 3. On-chain (Move / Sui) — **deploy target**

- ✅ `sui-move/` compiles (9 modules + `audit_registry`) on Sui Coin standard.
- ✅ Economic unit tests (`policy_tests.move`) prove split conservation.
- ✅ **`cput-sui` relayer crate** — adapters, JSON-RPC reconciliation, CLI submit,
  durable `cput-store` cursor.
- ✅ **`cput-relayer` binary** — end-to-end off-chain pipeline → `submit_report` +
  `execute_mint` (dry-run by default).
- ⚠️ **Addresses.** Fill `config/cput.example.toml` after `sui client publish`.
- ⚠️ **On-chain PQC verification.** ML-DSA/SLH-DSA are verified off-chain by the
  relayer; on-chain modules enforce threshold, replay, and economic invariants.
- ⚠️ **Transaction builder.** Relayer uses `sui client call` subprocess; migrate
  to `sui-sdk` PTB builder for HA deployments.
- ❌ **Upgradability & governance custody.** Package upgrade policy + DAO
  `AdminCap` transfer not yet automated.
- ⚠️ **Move Prover specs** — invariant targets documented in `sui-move/specs/README.md`;
  runtime tests in `policy_tests.move` pass; formal `sui move prove` not yet in CI.

### Legacy Aptos prototype (`move/`)

The original 11-module Aptos package remains for reference. New deployments
should use `sui-move/` only.

---

## 4. Security hardening checklist

- [ ] External audit of all economic invariants (split conservation, ceiling,
      burn routing, slashing math).
- [ ] Replace the reference ZK backend; audit the circuits.
- [ ] TEE attestation chain-of-trust verification (vendor root certs, TCB
      recency).
- [ ] DKG ceremony + threshold-signature scheme for the oracle DON.
- [ ] HSM-backed key custody; key-rotation runbooks.
- [ ] Persistent, replicated state for audit log and replay guards.
- [ ] Rate limiting / DoS protection on every gate ingress.
- [ ] Emergency circuit breaker wired to the `COUNCIL_THRESHOLD`-of-
      `COUNCIL_SET_SIZE` multisig.
- [ ] Monitoring & alerting on invariant violations (they should be impossible;
      alert if a gate ever rejects in production).

---

## 5. Deployment outline — **testnet first, mainnet after hardening**

> **Do not deploy to mainnet** until ZK, TEE, DKG, HSM, and Move Prover items
> in §4 are complete and externally audited.

### 5.1 Testnet (automated)

```powershell
# Fund testnet address first if needed: sui client faucet
.\scripts\deploy-testnet.ps1 -SuiCli ".\.sui-bin\sui.exe"
```

Then:

1. Copy emitted template → `config/cput.toml`; fill shared object IDs from publish output.
2. `cargo run --release --bin cput-agent -- --health` — quorum readiness probe.
3. `cargo run --release --bin cput-relayer -- --config config/cput.toml --dry-run`
4. Set `dry_run = false`; run one epoch; reconcile via `cput-sui::reconcile`.

### 5.2 Agent health (Layer 2)

Three **formula agents** (`MintingAgent` indices 0–2) each run the committed
emission policy (`DEFAULT_TOKENS_PER_GFLOP = 1`). Health checks:

| Check | Command / API |
|-------|----------------|
| Quorum size | `assess_quorum_health()` → `quorum_size == 3` |
| Spread within tolerance | `spread_bps <= 200` |
| Policy binding | `policy_hash_hex` matches on-chain governance hash |
| Readiness probe | `cput-agent --health` (exit 0 = healthy, JSON report) |
| Relayer preflight | `cput-relayer` aborts if quorum unhealthy before relay |

### 5.3 Mainnet gate (manual checklist)

- [ ] Reference ZK replaced with audited circuits
- [ ] TEE quote verification wired in L0
- [ ] Real DKG + HSM custody for oracle operators
- [ ] `sui move prove` passes conservation specs (P1–P4)
- [ ] Relayer migrated to `sui-sdk` PTB builder
- [ ] External economic audit complete
- [ ] `AdminCap` transferred to DAO multisig
