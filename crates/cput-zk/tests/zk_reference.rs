//! Integration tests: reference ZK backend prove/verify/aggregate pipeline.

use cput_core::ids::{EpochId, NodeId, WorkloadHash};
use cput_core::units::Gflops;
use cput_zk::{encode_statement, ComputeStatement, ProofBackend, ReferenceBackend};

#[test]
fn reference_prove_verify_roundtrip() {
    let backend = ReferenceBackend;
    let stmt = ComputeStatement {
        node_id: NodeId([0xAB; 32]),
        epoch: EpochId(1007),
        gflops: Gflops(15_236_250_000),
        workload_hash: WorkloadHash([0xCD; 32]),
        enclave_measurement: [0xEF; 32],
    };
    let inputs = encode_statement(&stmt).expect("encode");
    let proof = backend.prove(&inputs).expect("prove");
    backend.verify(&proof, &inputs).expect("verify");
    assert_eq!(proof.folded, 1);
    assert_eq!(proof.bytes, proof.public_inputs_commitment);
}

#[test]
fn reference_rejects_wrong_public_inputs() {
    let backend = ReferenceBackend;
    let inputs_a = b"statement-a";
    let inputs_b = b"statement-b";
    let proof = backend.prove(inputs_a).expect("prove");
    assert!(backend.verify(&proof, inputs_b).is_err());
}

#[test]
fn reference_aggregate_folds_node_proofs() {
    let backend = ReferenceBackend;
    let batch: Vec<Vec<u8>> = (0..3u8)
        .map(|i| encode_statement(&ComputeStatement {
            node_id: NodeId([i; 32]),
            epoch: EpochId(1007),
            gflops: Gflops(10_000_000_000 + u128::from(i)),
            workload_hash: WorkloadHash([i; 32]),
            enclave_measurement: [0x11; 32],
        }))
        .map(|r| r.expect("encode"))
        .collect();

    let leaves: Vec<_> = batch
        .iter()
        .map(|inputs| backend.prove(inputs).expect("prove"))
        .collect();
    let agg = backend.aggregate(&leaves).expect("aggregate");
    assert_eq!(agg.folded, 3);
    backend
        .verify_aggregate(&agg, &batch)
        .expect("verify aggregate");
}
