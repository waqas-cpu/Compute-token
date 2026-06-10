//! # cput-layer0-compute — VERTICAL LAYER 0
//!
//! Physical compute substrate: node registration & identity, hardware
//! telemetry, and TEE attestation. The layer's sole output is the
//! [`cput_gates::AttestationPacket`] consumed by Gate 0→1.
//!
//! ## Layer rules (invariants this layer guarantees)
//!
//! - **R0.1 Identity binding.** A node's [`NodeId`] is the SHA-256 digest of
//!   its committed ML-DSA-87 public key. The key is generated locally and only
//!   the public half leaves the node.
//! - **R0.2 Capacity bound.** Attested GFLOPs for an epoch may not exceed the
//!   node's rated capacity for the epoch duration (anti-overclaim).
//! - **R0.3 Sealed attestation.** Every attestation body is ML-DSA signed and
//!   accompanied by a ZK compute proof over the same public inputs.
//! - **R0.4 Telemetry sanity.** Thermal envelope is bounded; zero-GFLOP
//!   attestations are never produced.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod flop_meter;
pub mod tee;

pub use flop_meter::{SimulatedFlopMeter, SimulatedFlopMeterConfig};
pub use tee::{ReferenceTeeVerifier, TeeIdentity, TeeVerifier};

use cput_core::ids::{EpochId, NodeId, WorkloadHash};
use cput_core::policy::EPOCH_SECONDS;
use cput_core::units::Gflops;
use cput_core::{CputError, CputResult};
use cput_gates::contracts::{AttestationBody, AttestationPacket};
use cput_pqc::envelope::Signed;
use cput_pqc::mldsa::MlDsaKeypair;
use cput_zk::{ComputeStatement, ComputeWitness, ProofBackend};
use serde::{Deserialize, Serialize};

/// A point-in-time hardware telemetry sample (sampled at sub-epoch intervals).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySample {
    /// FLOP/s delivered at the sample instant.
    pub flops_per_sec: u128,
    /// VRAM utilisation in basis points.
    pub vram_util_bps: u16,
    /// Thermal envelope in degrees Celsius.
    pub thermal_c: u16,
    /// Instantaneous power draw in watts.
    pub power_w: u32,
}

/// A registered compute node with a TEE-bound ML-DSA identity key.
pub struct ComputeNode<'b, B: ProofBackend> {
    keypair: MlDsaKeypair,
    node_id: NodeId,
    /// Hardware class label (e.g. `"gpu-h100"`).
    pub hardware_class: String,
    /// Rated sustained compute, GFLOP/s.
    pub rated_gflops_per_sec: u128,
    /// Signed jurisdiction tag (for ERC-1400-style transfer restrictions).
    pub jurisdiction: String,
    /// Enclave measurement (MRENCLAVE) the node attests under.
    pub enclave_measurement: [u8; 32],
    backend: &'b B,
}

impl<'b, B: ProofBackend> ComputeNode<'b, B> {
    /// Register a new node, generating its TEE-bound ML-DSA-87 keypair.
    pub fn register(
        backend: &'b B,
        hardware_class: impl Into<String>,
        rated_gflops_per_sec: u128,
        jurisdiction: impl Into<String>,
        enclave_measurement: [u8; 32],
    ) -> CputResult<Self> {
        let keypair = MlDsaKeypair::generate()?;
        let node_id = AttestationBody::node_id_for_key(&keypair.public_bytes());
        Ok(Self {
            keypair,
            node_id,
            hardware_class: hardware_class.into(),
            rated_gflops_per_sec,
            jurisdiction: jurisdiction.into(),
            enclave_measurement,
            backend,
        })
    }

    /// This node's permanent identity (committed on-chain at registration).
    #[must_use]
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// The committed public key bytes.
    #[must_use]
    pub fn public_key(&self) -> Vec<u8> {
        self.keypair.public_bytes()
    }

    /// The maximum GFLOPs this node may attest for a single epoch (R0.2).
    #[must_use]
    pub fn epoch_capacity(&self) -> Gflops {
        Gflops(
            self.rated_gflops_per_sec
                .saturating_mul(u128::from(EPOCH_SECONDS)),
        )
    }

    /// Produce a sealed, ZK-proven attestation packet for an epoch (Layer 0
    /// output / producer side of Gate 0→1). Enforces R0.2 and R0.4.
    pub fn produce_attestation(
        &self,
        epoch: EpochId,
        gflops: Gflops,
        workload_hash: WorkloadHash,
        thermal_envelope_c: u16,
        verified_hash: [u8; 32],
    ) -> CputResult<AttestationPacket> {
        if gflops.0 == 0 {
            return Err(CputError::RuleViolation {
                rule: "R0.4",
                detail: "refusing to attest zero GFLOPs".into(),
            });
        }
        if gflops > self.epoch_capacity() {
            return Err(CputError::RuleViolation {
                rule: "R0.2",
                detail: format!(
                    "attested {} GFLOPs exceeds epoch capacity {}",
                    gflops.0,
                    self.epoch_capacity().0
                ),
            });
        }

        let body = AttestationBody {
            node_id: self.node_id,
            epoch,
            gflops,
            workload_hash,
            thermal_envelope_c,
            verified_hash,
            enclave_measurement: self.enclave_measurement,
        };

        // ZK compute proof over the public inputs (R0.3).
        let statement = ComputeStatement {
            node_id: self.node_id,
            epoch,
            gflops,
            workload_hash,
            enclave_measurement: self.enclave_measurement,
        };
        let witness =
            ComputeWitness::from_attestation(gflops, workload_hash, verified_hash);
        let compute_proof = self.backend.prove_compute(&statement, &witness)?;

        // Seal the body under the node's ML-DSA key (R0.3).
        let signed = Signed::seal_mldsa(&self.keypair, body)?;
        Ok(AttestationPacket {
            signed,
            compute_proof,
        })
    }

    /// Produce an attestation from flop-meter telemetry samples (simulated or real).
    ///
    /// Aggregates sample throughput into epoch GFLOPs and uses the peak thermal
    /// reading across the window. Enforces R0.2 and R0.4 via [`produce_attestation`].
    pub fn produce_attestation_from_samples(
        &self,
        epoch: EpochId,
        samples: &[TelemetrySample],
        sample_interval_secs: u64,
        workload_hash: WorkloadHash,
        verified_hash: [u8; 32],
    ) -> CputResult<AttestationPacket> {
        if samples.is_empty() {
            return Err(CputError::RuleViolation {
                rule: "R0.4",
                detail: "refusing to attest with zero telemetry samples".into(),
            });
        }
        let gflops = SimulatedFlopMeter::aggregate_gflops(samples, sample_interval_secs);
        let thermal_envelope_c = SimulatedFlopMeter::peak_thermal_c(samples);
        self.produce_attestation(
            epoch,
            gflops,
            workload_hash,
            thermal_envelope_c,
            verified_hash,
        )
    }
}
