//! Gate 0→1: ML-DSA attestation + reference ZK compute proof must both verify.

use cput_core::ids::{EpochId, WorkloadHash};
use cput_core::units::Gflops;
use cput_gates::gate::{admit_attestation, GateConfig};
use cput_layer0_compute::ComputeNode;
use cput_pqc::AlgorithmRegistry;
use cput_zk::ReferenceBackend;

#[test]
fn gate_admits_signed_attestation_with_zk_proof() {
    let registry = AlgorithmRegistry::default();
    let backend = ReferenceBackend;
    let cfg = GateConfig {
        registry: &registry,
        backend: &backend,
    };

    let node = ComputeNode::register(&backend, "gpu-h100", 50_000_000, "US", [1; 32])
        .expect("register");
    let packet = node
        .produce_attestation(
            EpochId(1007),
            Gflops(15_236_250_000),
            WorkloadHash([0xAB; 32]),
            68,
            [0xCD; 32],
        )
        .expect("attest");

    assert!(!packet.signed.envelope.signature.is_empty());
    assert_eq!(packet.compute_proof.folded, 1);
    admit_attestation(&cfg, &packet).expect("gate must admit valid PQC+ZK packet");
}
