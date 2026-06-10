//! Gate 0→1 with real Groth16 SNARK proofs.

use cput_core::ids::{EpochId, WorkloadHash};
use cput_core::units::Gflops;
use cput_gates::gate::{admit_attestation, GateConfig};
use cput_layer0_compute::ComputeNode;
use cput_pqc::AlgorithmRegistry;
use cput_zk::SnarkBackend;

#[test]
fn gate_admits_groth16_attestation() {
    let registry = AlgorithmRegistry::default();
    let backend = SnarkBackend;
    let cfg = GateConfig {
        registry: &registry,
        backend: &backend,
    };
    let node = ComputeNode::register(&backend, "gpu-h100", 50_000_000, "US", [9; 32])
        .expect("register");
    let packet = node
        .produce_attestation(
            EpochId(3001),
            Gflops(12_000_000_000),
            WorkloadHash([0xAB; 32]),
            70,
            [0xCD; 32],
        )
        .expect("attest");
    admit_attestation(&cfg, &packet).expect("gate admits groth16 attestation");
}
