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
- ⚠️ **Key management.** Keys are generated in-process. Production must bind
  ML-DSA signing keys to the node's TEE (sealing/quoting), store oracle DKG
  shares in HSMs, and rotate per `DKG_ROTATION_EPOCHS`.
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
| L0 compute | ❌ Real TEE attestation (SGX/SEV-SNP/TDX quote verification) instead of the placeholder enclave measurement. ⚠️ Hardware telemetry source. |
| L1 oracle  | ❌ Real DKG + threshold signatures (currently independent ML-DSA sigs counted to a threshold). ❌ Challenge-sampling protocol & dispute handling. |
| L2 agentic | ⚠️ Agents share one deterministic formula; production runs genuinely independent models/inference and compares. ❌ Byzantine-agent eviction. |
| L4 tokenomics | ⚠️ TWAP utilisation is an input; production must source it from real demand telemetry. ⚠️ State is in-memory; back it with the chain as source of truth. |
| L5 settlement | ⚠️ Governance is single-process; production binds it to the on-chain `governance` module and a real proposal lifecycle (timelocks, quorums). |

---

## 3. On-chain (Move / Aptos)

- ✅ All 11 modules compile against the Aptos mainnet framework.
- ⚠️ **Address.** `move/Move.toml` pins `cput = 0xc9007` for compile convenience.
  Deploy under a real resource account and switch to a named-address profile.
- ❌ **On-chain PQC verification.** ML-DSA/SLH-DSA are not in the Aptos framework.
  Options: (a) a verified off-chain relayer that the framework trusts via a
  multisig, (b) a custom native function / precompile, (c) a Move-native
  verifier if/when one is standardised. Today the Move gates enforce the
  *threshold, replay, and economic* invariants and treat signature bytes as
  relayer-checked.
- ❌ **Token framework.** `cput_token` is a self-contained balance ledger for
  clarity. Production should migrate to the Aptos **Dispatchable Fungible Asset**
  standard so wallets/DEXes interoperate, keeping the `compliance` hook as the
  transfer dispatch function.
- ❌ **Upgradability & governance custody.** Use resource-account-based package
  publishing with governance-controlled upgrade policy.
- ❌ **Tests & formal verification.** Add Move unit tests and Move Prover
  specs (`spec` blocks) for the conservation and threshold invariants. *(Out of
  scope for this prototype per the build request.)*

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

## 5. Deployment outline

1. Stand up oracle DON with DKG; publish operator key set to `oracle_verifier`.
2. Publish the Move package under a resource account; `initialize` every module
   (`pqc_registry`, `audit_registry`, `compliance`, `cput_token`, `governor`,
   `minting` pools, `staking`, `governance`).
3. Deploy L0 node binaries into TEEs; register nodes.
4. Run one epoch end-to-end on testnet; reconcile the off-chain
   `SettlementReceipt` against the on-chain `EpochMinted` event.
5. Enable governance; transfer admin authority to the DAO/council multisig.
