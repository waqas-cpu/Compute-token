# Move Prover specs (conservation invariants)

Formal verification targets for `cput::policy` conservation laws. The runtime
unit tests in `tests/policy_tests.move` already exercise these invariants; Move
Prover specs will lock them before mainnet.

## Invariants to prove

| ID | Function | Property |
|----|----------|----------|
| P1 | `policy::compute_distribution` | `providers + oracle + treasury + burn_reserve == total` |
| P2 | `policy::compute_fee_routing` | `burned + providers + treasury == fee` |
| P3 | `policy::recommend_ceiling` | `recommend_ceiling(c, u) <= c` when `u < UTIL_LOW_BPS` |
| P4 | `policy::recommend_ceiling` | `recommend_ceiling(c, u) >= c` when `u > UTIL_HIGH_BPS` |

## Run (when prover toolchain is wired)

```bash
cd sui-move
sui move prove
```

Requires Sui CLI with Move Prover enabled and `#[spec_only]` modules. Until the
prover is integrated in CI, rely on `sui move test` (4 conservation tests).

## Planned spec module

Add `specs/policy_specs.move` with `#[spec_only]` blocks mirroring P1–P4 once
the pinned framework version supports `sui move prove` on Windows.
