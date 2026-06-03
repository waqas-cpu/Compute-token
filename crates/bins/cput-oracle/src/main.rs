//! Oracle DON daemon demonstration (Layer 1).
//!
//! Stands up a 9-operator decentralised oracle network, ingests a batch of
//! Layer 0 attestations through **Gate 0→1**, recursively aggregates their ZK
//! proofs, and emits the threshold-signed epoch report consumed by Layer 2.

use cput_core::ids::{EpochId, OracleId, WorkloadHash};
use cput_core::policy;
use cput_core::units::Gflops;
use cput_layer0_compute::ComputeNode;
use cput_layer1_oracle::{OracleNetwork, OracleNode};
use cput_pqc::AlgorithmRegistry;
use cput_zk::ReferenceBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = AlgorithmRegistry::default();
    let backend = ReferenceBackend;
    let epoch = EpochId(42);

    println!("== CPUT oracle DON (Layer 1) ==");
    println!(
        "threshold = {}-of-{}\n",
        policy::ORACLE_THRESHOLD,
        policy::ORACLE_SET_SIZE
    );

    // Synthesise a batch of Layer 0 attestations to ingest.
    let mut attestations = Vec::new();
    for i in 0..4u8 {
        let node = ComputeNode::register(&backend, "gpu-h100", 50_000_000, "US", [i; 32])?;
        let packet = node.produce_attestation(
            epoch,
            Gflops(10_000_000 + u128::from(i) * 1_000_000),
            WorkloadHash([i; 32]),
            68,
            [0xCD; 32],
        )?;
        attestations.push(packet);
    }

    let operators: Vec<OracleNode> = (0..policy::ORACLE_SET_SIZE as u16)
        .map(OracleId)
        .map(OracleNode::new)
        .collect::<Result<_, _>>()?;
    let don = OracleNetwork::new(&registry, &backend, operators);

    let report = don.finalize_epoch(epoch, &attestations)?;
    println!(
        "ingested {} attestations through Gate 0→1",
        attestations.len()
    );
    println!(
        "verified total: {} GFLOPs",
        report.body.verified_gflops_total.0
    );
    println!(
        "aggregate proof folds {} node proofs",
        report.aggregated_proof.folded
    );
    println!(
        "zk commitment: {}",
        hex::encode(report.body.zk_proof_commitment)
    );
    println!(
        "threshold signatures collected: {}",
        report.threshold_sig.shares.len()
    );
    println!(
        "dispute window closes at {}",
        report.body.dispute_window_close
    );
    Ok(())
}
