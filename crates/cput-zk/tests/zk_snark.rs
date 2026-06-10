//! Groth16 SNARK integration tests (real prove + verify).

use cput_core::ids::{EpochId, NodeId, WorkloadHash};
use cput_core::units::Gflops;
use cput_zk::{
    encode_statement, AgentReasoningStatement, ComputeStatement, ComputeWitness,
    ProofBackend, ProofSystemId, ReasoningWitness, SnarkBackend,
};

#[test]
fn groth16_compute_prove_verify() {
    let backend = SnarkBackend;
    let statement = ComputeStatement {
        node_id: NodeId([0x11; 32]),
        epoch: EpochId(2001),
        gflops: Gflops(25_000_000_000),
        workload_hash: WorkloadHash([0x22; 32]),
        enclave_measurement: [0x33; 32],
    };
    let witness = ComputeWitness::from_attestation(
        statement.gflops,
        statement.workload_hash,
        [0x44; 32],
    );
    let proof = backend
        .prove_compute(&statement, &witness)
        .expect("prove compute");
    assert_eq!(proof.system, ProofSystemId::Groth16);
    assert!(proof.bytes.len() > 32, "groth16 proof must be non-trivial");
    let inputs = encode_statement(&statement).expect("encode");
    backend.verify(&proof, &inputs).expect("verify compute");
}

#[test]
fn groth16_reasoning_prove_verify() {
    let backend = SnarkBackend;
    let statement = AgentReasoningStatement {
        epoch: EpochId(2001),
        verified_gflops: Gflops(25_000_000_000),
        mint_amount: 25_000_000_000,
        policy_hash: [0x55; 32],
    };
    let witness = ReasoningWitness::from_formula(2001, 25_000_000_000, 1, 25_000_000_000);
    let proof = backend
        .prove_reasoning(&statement, &witness)
        .expect("prove reasoning");
    assert_eq!(proof.system, ProofSystemId::Groth16);
    let inputs = encode_statement(&statement).expect("encode");
    backend.verify(&proof, &inputs).expect("verify reasoning");
}

#[test]
fn groth16_aggregate_batch_verify() {
    let backend = SnarkBackend;
    let mut proofs = Vec::new();
    let mut batch = Vec::new();
    for i in 0..3u8 {
        let statement = ComputeStatement {
            node_id: NodeId([i; 32]),
            epoch: EpochId(2002),
            gflops: Gflops(10_000_000_000 + u128::from(i)),
            workload_hash: WorkloadHash([i; 32]),
            enclave_measurement: [0xEE; 32],
        };
        let witness = ComputeWitness::from_attestation(
            statement.gflops,
            statement.workload_hash,
            [i; 32],
        );
        let proof = backend
            .prove_compute(&statement, &witness)
            .expect("prove");
        batch.push(encode_statement(&statement).expect("encode"));
        proofs.push(proof);
    }
    let agg = backend.aggregate(&proofs).expect("aggregate");
    backend
        .verify_aggregate(&agg, &batch)
        .expect("verify aggregate");
    assert_eq!(agg.folded, 3);
}
