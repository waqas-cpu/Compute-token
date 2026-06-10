//! Integration gates: the validation rules enforced at each layer boundary.
//!
//! A gate is a *checkpoint*. A payload may only cross from one layer to the
//! next if it satisfies every rule the gate enforces: PQC signature validity,
//! crypto-agility approval, ZK-proof validity, threshold quorum, and the
//! structural / economic invariants of the contract. Gates never trust
//! upstream data — they re-verify it.

use crate::contracts::{
    AttestationBody, AttestationPacket, EpochReport, MintInstruction, PolicyUpdate,
    SettlementReceipt,
};
use cput_core::ids::OracleId;
use cput_core::units::{Gflops, TokenAmount};
use cput_core::{policy, CputError, CputResult};
use cput_pqc::AlgorithmRegistry;
use cput_zk::{encode_statement, AgentReasoningStatement, ComputeStatement, ProofBackend};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

fn canonical<T: Serialize>(body: &T) -> CputResult<Vec<u8>> {
    serde_json::to_vec(body).map_err(|e| CputError::Codec(format!("gate canonical encode: {e}")))
}

/// Shared gate configuration: the crypto-agility registry and ZK backend.
pub struct GateConfig<'a, B: ProofBackend> {
    /// Approved-algorithm registry (crypto-agility).
    pub registry: &'a AlgorithmRegistry,
    /// ZK proof backend used to verify proofs.
    pub backend: &'a B,
}

/// **Gate 0→1** — admit a per-node compute attestation into the oracle layer.
///
/// Rules: (1) ML-DSA signature valid & approved; (2) the node identity binds to
/// the signing key (`node_id == SHA-256(pubkey)`); (3) telemetry within sane
/// bounds; (4) the ZK compute proof verifies against the attested public inputs.
pub fn admit_attestation<B: ProofBackend>(
    cfg: &GateConfig<'_, B>,
    packet: &AttestationPacket,
) -> CputResult<()> {
    const GATE: &str = "GATE.0->1.ATTESTATION";

    packet.signed.verify(cfg.registry)?;

    let derived = AttestationBody::node_id_for_key(packet.signed.signer_public());
    if derived != packet.signed.body.node_id {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "node_id is not the digest of the signing key".into(),
        });
    }

    let body = &packet.signed.body;
    if body.gflops.0 == 0 {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "zero GFLOPs attested".into(),
        });
    }
    if body.thermal_envelope_c > 150 {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: format!("implausible thermal envelope {}°C", body.thermal_envelope_c),
        });
    }

    let statement = ComputeStatement {
        node_id: body.node_id,
        epoch: body.epoch,
        gflops: body.gflops,
        workload_hash: body.workload_hash,
        enclave_measurement: body.enclave_measurement,
    };
    let inputs = encode_statement(&statement)?;
    cfg.backend
        .verify(&packet.compute_proof, &inputs)
        .map_err(|e| CputError::GateRejected {
            gate: GATE,
            reason: e.to_string(),
        })?;
    Ok(())
}

/// **Gate 1→2** — admit a threshold-signed epoch report into the agentic layer.
///
/// Rules: (1) at least `ORACLE_THRESHOLD` distinct, known oracles signed the
/// exact body; (2) the aggregate proof commitment matches the body and folds
/// one proof per node; (3) GFLOP totals reconcile; (4) the dispute window is
/// set per policy.
pub fn admit_epoch_report<B: ProofBackend>(
    cfg: &GateConfig<'_, B>,
    oracle_keys: &BTreeMap<OracleId, Vec<u8>>,
    report: &EpochReport,
) -> CputResult<()> {
    const GATE: &str = "GATE.1->2.EPOCH_REPORT";

    // (1) Threshold signature over the canonical body.
    let body_bytes = canonical(&report.body)?;
    let mut signers: BTreeSet<OracleId> = BTreeSet::new();
    for (oracle_id, env) in &report.threshold_sig.shares {
        let Some(expected_key) = oracle_keys.get(oracle_id) else {
            return Err(CputError::GateRejected {
                gate: GATE,
                reason: format!("unknown oracle id {oracle_id:?}"),
            });
        };
        if env.payload != body_bytes {
            return Err(CputError::GateRejected {
                gate: GATE,
                reason: "oracle share does not cover the report body".into(),
            });
        }
        if &env.signer_public != expected_key {
            return Err(CputError::GateRejected {
                gate: GATE,
                reason: format!("oracle {oracle_id:?} signed with an unregistered key"),
            });
        }
        env.verify(cfg.registry)
            .map_err(|e| CputError::GateRejected {
                gate: GATE,
                reason: e.to_string(),
            })?;
        signers.insert(*oracle_id);
    }
    if signers.len() < policy::ORACLE_THRESHOLD {
        return Err(CputError::ThresholdNotMet {
            have: signers.len(),
            need: policy::ORACLE_THRESHOLD,
        });
    }

    // (2) Aggregate proof binds to the body and folds one proof per node.
    if report.aggregated_proof.public_inputs_commitment != report.body.zk_proof_commitment {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "aggregate proof commitment != body.zk_proof_commitment".into(),
        });
    }
    if report.aggregated_proof.folded as usize != report.body.node_scores.len() {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "aggregate proof folds a different number of proofs than node scores".into(),
        });
    }
    if report.leaf_public_inputs.len() != report.body.node_scores.len() {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "leaf_public_inputs length != node_scores length".into(),
        });
    }
    cfg.backend
        .verify_aggregate(&report.aggregated_proof, &report.leaf_public_inputs)
        .map_err(|e| CputError::GateRejected {
            gate: GATE,
            reason: format!("aggregate ZK proof invalid: {e}"),
        })?;

    // (3) Reconcile GFLOP totals.
    if report.body.node_scores.is_empty() {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "empty epoch report".into(),
        });
    }
    let summed = report
        .body
        .node_scores
        .iter()
        .fold(Gflops(0), |acc, s| acc.saturating_add(s.gflops));
    if summed != report.body.verified_gflops_total {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "node GFLOP scores do not sum to verified_gflops_total".into(),
        });
    }

    // (4) Dispute window policy.
    let expected_close =
        cput_core::ids::EpochId(report.body.epoch.0 + policy::DISPUTE_WINDOW_EPOCHS);
    if report.body.dispute_window_close != expected_close {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "dispute window close does not match policy".into(),
        });
    }
    Ok(())
}

/// **Gate 2→4** — admit a signed mint instruction into the tokenomics engine.
///
/// Rules: (1) ML-DSA signature valid & from the registered coordinating agent;
/// (2) allocations sum to the total; (3) the total respects the current mint
/// ceiling (anti-inflation); (4) the ZK reasoning proof verifies against the
/// epoch's verified GFLOPs and committed policy.
pub fn admit_mint_instruction<B: ProofBackend>(
    cfg: &GateConfig<'_, B>,
    expected_agent_key: &[u8],
    verified_gflops: Gflops,
    mint_ceiling: TokenAmount,
    instruction: &MintInstruction,
) -> CputResult<()> {
    const GATE: &str = "GATE.2->4.MINT_INSTRUCTION";

    instruction.signed.verify(cfg.registry)?;
    if instruction.signed.signer_public() != expected_agent_key {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "mint instruction not signed by the registered coordinator key".into(),
        });
    }

    // R2.2 — each quorum agent must have a valid ML-DSA proposal signature.
    if instruction.agent_proposals.len() < policy::AGENT_QUORUM_SIZE {
        return Err(CputError::ThresholdNotMet {
            have: instruction.agent_proposals.len(),
            need: policy::AGENT_QUORUM_SIZE,
        });
    }
    let body = &instruction.signed.body;
    let mut proposals: Vec<u128> = Vec::with_capacity(instruction.agent_proposals.len());
    for signed_proposal in &instruction.agent_proposals {
        signed_proposal.verify(cfg.registry).map_err(|e| CputError::GateRejected {
            gate: GATE,
            reason: format!("agent proposal signature invalid: {e}"),
        })?;
        let pb = &signed_proposal.body;
        if pb.epoch != body.epoch {
            return Err(CputError::GateRejected {
                gate: GATE,
                reason: "agent proposal epoch mismatch".into(),
            });
        }
        if pb.policy_hash != body.policy_hash {
            return Err(CputError::GateRejected {
                gate: GATE,
                reason: "agent proposal policy_hash mismatch".into(),
            });
        }
        proposals.push(pb.proposal);
    }
    if !proposals.is_empty() {
        let min = *proposals.iter().min().expect("non-empty");
        let max = *proposals.iter().max().expect("non-empty");
        if min > 0 {
            let spread_bps = ((max - min).saturating_mul(10_000) / min) as u32;
            if spread_bps > u32::from(policy::CONSENSUS_TOLERANCE_BPS.0) {
                return Err(CputError::ConsensusFailed {
                    spread_bps,
                    tolerance_bps: u32::from(policy::CONSENSUS_TOLERANCE_BPS.0),
                });
            }
        }
    }

    let alloc_sum = body
        .allocations
        .iter()
        .fold(TokenAmount(0), |acc, a| acc.saturating_add(a.amount));
    if alloc_sum != body.total {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "per-node allocations do not sum to total".into(),
        });
    }

    if body.total.0 > mint_ceiling.0 {
        return Err(CputError::CircuitBreaker(format!(
            "mint total {} exceeds ceiling {}",
            body.total.0, mint_ceiling.0
        )));
    }

    let statement = AgentReasoningStatement {
        epoch: body.epoch,
        verified_gflops,
        mint_amount: body.total.0,
        policy_hash: body.policy_hash,
    };
    let inputs = encode_statement(&statement)?;
    cfg.backend
        .verify(&instruction.reasoning_proof, &inputs)
        .map_err(|e| CputError::GateRejected {
            gate: GATE,
            reason: e.to_string(),
        })?;
    Ok(())
}

/// **Gate 4→5** — admit a settlement receipt into the settlement layer.
///
/// Rules: (1) signature valid; (2) the four distribution shares sum to the
/// minted total; (3) each share equals the policy split of the minted total
/// (rounding remainder folded into the provider share).
pub fn admit_settlement_receipt(
    registry: &AlgorithmRegistry,
    receipt: &SettlementReceipt,
) -> CputResult<()> {
    const GATE: &str = "GATE.4->5.SETTLEMENT";

    receipt.verify(registry)?;
    let b = &receipt.body;

    let sum = b
        .providers
        .saturating_add(b.oracle)
        .saturating_add(b.treasury)
        .saturating_add(b.burn_reserve);
    if sum != b.minted_total {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "distribution shares do not sum to minted_total".into(),
        });
    }

    let oracle_expected = b.minted_total.apply_bps(policy::SPLIT_ORACLE_BPS);
    let treasury_expected = b.minted_total.apply_bps(policy::SPLIT_TREASURY_BPS);
    let burn_expected = b.minted_total.apply_bps(policy::SPLIT_BURN_RESERVE_BPS);
    let providers_expected = TokenAmount(
        b.minted_total
            .0
            .saturating_sub(oracle_expected.0)
            .saturating_sub(treasury_expected.0)
            .saturating_sub(burn_expected.0),
    );
    if b.oracle != oracle_expected
        || b.treasury != treasury_expected
        || b.burn_reserve != burn_expected
        || b.providers != providers_expected
    {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "distribution shares do not match policy split".into(),
        });
    }
    Ok(())
}

/// **Downward gate 5→2/4** — admit a governance policy update into the minting
/// parameters.
///
/// Rules: (1) SLH-DSA governance signature valid & from the registered
/// governance key; (2) a non-zero mint ceiling unless the circuit breaker is
/// engaged.
pub fn admit_policy_update(
    registry: &AlgorithmRegistry,
    governance_key: &[u8],
    update: &PolicyUpdate,
) -> CputResult<()> {
    const GATE: &str = "GATE.5->2/4.POLICY";

    update.verify(registry)?;
    if update.signer_public() != governance_key {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "policy update not signed by the governance key".into(),
        });
    }
    if update.body.mint_ceiling.0 == 0 && !update.body.circuit_breaker_engaged {
        return Err(CputError::GateRejected {
            gate: GATE,
            reason: "zero mint ceiling without engaging the circuit breaker".into(),
        });
    }
    Ok(())
}
