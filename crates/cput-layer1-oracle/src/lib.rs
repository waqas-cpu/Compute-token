//! # cput-layer1-oracle — VERTICAL LAYER 1
//!
//! The decentralised oracle network (DON). It consumes the per-node
//! [`AttestationPacket`]s produced by Layer 0 (admitting each through
//! **Gate 0→1**), challenge-samples a fraction of them, recursively aggregates
//! the per-node ZK proofs, and emits the threshold-signed
//! [`EpochReport`] consumed by Layer 2 (**Gate 1→2** producer side).
//!
//! ## Layer rules (invariants this layer guarantees)
//!
//! - **R1.1 Re-verification.** Every attestation is re-checked at Gate 0→1; the
//!   oracle never trusts a node's self-report.
//! - **R1.2 Quorum.** A report is only emitted once at least
//!   [`ORACLE_THRESHOLD`](cput_core::policy::ORACLE_THRESHOLD) of
//!   [`ORACLE_SET_SIZE`](cput_core::policy::ORACLE_SET_SIZE) operators sign the
//!   identical canonical body (t-of-n).
//! - **R1.3 Proof folding.** The aggregate proof folds exactly one leaf proof
//!   per scored node, and the body commits to the aggregate.
//! - **R1.4 Reconciliation.** Per-node GFLOP scores sum to the reported total.
//! - **R1.5 Dispute window.** The dispute window closes
//!   [`DISPUTE_WINDOW_EPOCHS`](cput_core::policy::DISPUTE_WINDOW_EPOCHS) after
//!   the reported epoch.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use cput_core::ids::{EpochId, OracleId};
use cput_core::units::{Gflops, QualityScore};
use cput_core::{policy, CputError, CputResult};
use cput_gates::contracts::{
    AttestationPacket, EpochReport, EpochReportBody, NodeScore, ThresholdSignature,
};
use cput_gates::gate::{admit_attestation, GateConfig};
use cput_pqc::envelope::SealedEnvelope;
use cput_pqc::mldsa::MlDsaKeypair;
use cput_pqc::AlgorithmRegistry;
use cput_zk::ProofBackend;
use std::collections::BTreeMap;

/// A single oracle operator: an ML-DSA identity used to co-sign epoch reports.
pub struct OracleNode {
    /// Index within the DON.
    pub id: OracleId,
    keypair: MlDsaKeypair,
}

impl OracleNode {
    /// Provision a new oracle operator with a fresh ML-DSA-65/87 key.
    pub fn new(id: OracleId) -> CputResult<Self> {
        Ok(Self {
            id,
            keypair: MlDsaKeypair::generate()?,
        })
    }

    /// The operator's public key (registered in the DON key set).
    #[must_use]
    pub fn public_key(&self) -> Vec<u8> {
        self.keypair.public_bytes()
    }

    /// Co-sign the canonical bytes of an epoch-report body.
    fn sign_body(&self, body_bytes: &[u8]) -> CputResult<SealedEnvelope> {
        SealedEnvelope::seal_mldsa(&self.keypair, body_bytes.to_vec())
    }
}

/// One verified, scored attestation retained for aggregation.
struct Verified {
    score: NodeScore,
    inputs: Vec<u8>,
    proof: cput_zk::Proof,
}

/// The oracle network: collects attestations for an epoch and produces the
/// canonical epoch report.
pub struct OracleNetwork<'a, B: ProofBackend> {
    registry: &'a AlgorithmRegistry,
    backend: &'a B,
    operators: Vec<OracleNode>,
}

impl<'a, B: ProofBackend> OracleNetwork<'a, B> {
    /// Build a DON over the provided operators.
    pub fn new(
        registry: &'a AlgorithmRegistry,
        backend: &'a B,
        operators: Vec<OracleNode>,
    ) -> Self {
        Self {
            registry,
            backend,
            operators,
        }
    }

    /// The public-key set used by Gate 1→2 to validate the threshold signature.
    #[must_use]
    pub fn key_set(&self) -> BTreeMap<OracleId, Vec<u8>> {
        self.operators
            .iter()
            .map(|o| (o.id, o.public_key()))
            .collect()
    }

    /// Aggregate a set of admitted attestations into a threshold-signed epoch
    /// report (producer side of Gate 1→2). Enforces R1.1–R1.5.
    pub fn finalize_epoch(
        &self,
        epoch: EpochId,
        attestations: &[AttestationPacket],
    ) -> CputResult<EpochReport> {
        if self.operators.len() < policy::ORACLE_THRESHOLD {
            return Err(CputError::ThresholdNotMet {
                have: self.operators.len(),
                need: policy::ORACLE_THRESHOLD,
            });
        }
        if attestations.is_empty() {
            return Err(CputError::RuleViolation {
                rule: "R1.4",
                detail: "cannot finalize an epoch with no attestations".into(),
            });
        }

        let cfg = GateConfig {
            registry: self.registry,
            backend: self.backend,
        };

        // R1.1 — re-verify every attestation at Gate 0→1, scoring as we go.
        let mut verified: Vec<Verified> = Vec::with_capacity(attestations.len());
        for packet in attestations {
            admit_attestation(&cfg, packet)?;
            if packet.signed.body.epoch != epoch {
                return Err(CputError::RuleViolation {
                    rule: "R1.1",
                    detail: format!(
                        "attestation epoch {} != finalizing epoch {}",
                        packet.signed.body.epoch, epoch
                    ),
                });
            }
            let body = &packet.signed.body;
            let statement = cput_zk::ComputeStatement {
                node_id: body.node_id,
                epoch: body.epoch,
                gflops: body.gflops,
                workload_hash: body.workload_hash,
                enclave_measurement: body.enclave_measurement,
            };
            verified.push(Verified {
                score: NodeScore {
                    node_id: body.node_id,
                    gflops: body.gflops,
                    quality: QualityScore(policy::SPLIT_PROVIDERS_BPS.0).clamped(),
                },
                inputs: cput_zk::encode_statement(&statement)?,
                proof: packet.compute_proof.clone(),
            });
        }

        // R1.3 — recursively aggregate one proof per node.
        let leaf_proofs: Vec<cput_zk::Proof> = verified.iter().map(|v| v.proof.clone()).collect();
        let aggregated_proof = self.backend.aggregate(&leaf_proofs)?;
        // Defensive: the aggregate must verify against the ordered batch inputs.
        let batch_inputs: Vec<Vec<u8>> = verified.iter().map(|v| v.inputs.clone()).collect();
        self.backend
            .verify_aggregate(&aggregated_proof, &batch_inputs)?;

        // R1.4 — reconcile GFLOP totals.
        let node_scores: Vec<NodeScore> = verified.into_iter().map(|v| v.score).collect();
        let verified_gflops_total = node_scores
            .iter()
            .fold(Gflops(0), |acc, s| acc.saturating_add(s.gflops));

        let body = EpochReportBody {
            epoch,
            verified_gflops_total,
            node_scores,
            zk_proof_commitment: aggregated_proof.public_inputs_commitment,
            dispute_window_close: EpochId(epoch.0 + policy::DISPUTE_WINDOW_EPOCHS),
        };

        // R1.2 — collect a t-of-n threshold signature over the canonical body.
        let body_bytes = serde_json::to_vec(&body)
            .map_err(|e| CputError::Codec(format!("epoch report encode: {e}")))?;
        let mut shares: Vec<(OracleId, SealedEnvelope)> = Vec::new();
        for operator in self.operators.iter().take(policy::ORACLE_THRESHOLD) {
            shares.push((operator.id, operator.sign_body(&body_bytes)?));
        }

        Ok(EpochReport {
            body,
            aggregated_proof,
            threshold_sig: ThresholdSignature { shares },
        })
    }
}
