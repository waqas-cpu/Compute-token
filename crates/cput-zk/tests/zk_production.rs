//! Production backend: witness binding and constraint enforcement.

use cput_core::ids::{EpochId, NodeId, WorkloadHash};
use cput_core::units::Gflops;
use cput_zk::{
    encode_statement, AgentReasoningStatement, ComputeStatement, ComputeWitness,
    ProductionBackend, ProofBackend, ProofSystemId, ReasoningWitness,
};

#[test]
fn production_compute_binds_witness() {
    let backend = ProductionBackend;
    let statement = ComputeStatement {
        node_id: NodeId([1; 32]),
        epoch: EpochId(1008),
        gflops: Gflops(10_000_000),
        workload_hash: WorkloadHash([2; 32]),
        enclave_measurement: [3; 32],
    };
    let witness = ComputeWitness::from_attestation(
        statement.gflops,
        statement.workload_hash,
        [0xAA; 32],
    );
    let proof = backend
        .prove_compute(&statement, &witness)
        .expect("prove");
    assert_eq!(proof.system, ProofSystemId::UltraPlonk);
    assert_eq!(proof.bytes.len(), 64);
    let inputs = encode_statement(&statement).expect("encode");
    backend.verify(&proof, &inputs).expect("verify");
}

#[test]
fn production_rejects_gflop_mismatch() {
    let backend = ProductionBackend;
    let statement = ComputeStatement {
        node_id: NodeId([1; 32]),
        epoch: EpochId(1008),
        gflops: Gflops(10_000_000),
        workload_hash: WorkloadHash([2; 32]),
        enclave_measurement: [3; 32],
    };
    let mut witness = ComputeWitness::from_attestation(
        statement.gflops,
        statement.workload_hash,
        [0xAA; 32],
    );
    witness.measured_gflops += 1;
    assert!(backend.prove_compute(&statement, &witness).is_err());
}

#[test]
fn production_reasoning_binds_formula_output() {
    let backend = ProductionBackend;
    let statement = AgentReasoningStatement {
        epoch: EpochId(1008),
        verified_gflops: Gflops(33_000_000),
        mint_amount: 33_000_000,
        policy_hash: [0xAB; 32],
    };
    let witness = ReasoningWitness::from_formula(1008, 33_000_000, 1, 33_000_000);
    let proof = backend
        .prove_reasoning(&statement, &witness)
        .expect("prove");
    let inputs = encode_statement(&statement).expect("encode");
    backend.verify(&proof, &inputs).expect("verify");
}
