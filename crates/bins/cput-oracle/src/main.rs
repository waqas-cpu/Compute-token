//! Oracle DON daemon (Layer 1).
//!
//! - **Default:** synthesise attestations in-process (demo).
//! - **`--ingest`:** read attestation packets from the pipeline queue, emit a
//!   threshold-signed epoch report, and push to the epoch-reports topic.

mod ingest;

use clap::Parser;
use cput_core::ids::{EpochId, OracleId, WorkloadHash};
use cput_core::policy;
use cput_core::units::Gflops;
use cput_layer0_compute::ComputeNode;
use cput_layer1_oracle::{OracleNetwork, OracleNode};
use cput_pqc::AlgorithmRegistry;
use cput_sui::submit_report_from_epoch;
use cput_zk::ZkBackend;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "cput-oracle")]
struct Args {
    #[arg(long, default_value_t = 42)]
    epoch: u64,
    #[arg(long)]
    relay_config: Option<PathBuf>,
    /// Ingest attestations from the pipeline queue.
    #[arg(long)]
    ingest: bool,
    /// Pipeline queue directory.
    #[arg(long, default_value = "data/pipeline")]
    queue: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.ingest {
        return run_ingest(&args);
    }
    run_demo(&args)
}

fn run_ingest(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let epoch = EpochId(args.epoch);
    println!("== cput-oracle ingest epoch {} ==", epoch.0);
    let report = ingest::ingest_attestations(&args.queue, epoch)?;
    let intent = submit_report_from_epoch(&report)?;
    println!(
        "sui submit_report intent: epoch={} gflops={} signers={}",
        intent.epoch,
        intent.verified_gflops,
        intent.signer_indices.len()
    );
    println!("== oracle ingest complete ==");
    Ok(())
}

fn run_demo(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let registry = AlgorithmRegistry::default();
    let backend = ZkBackend::from_env();
    let epoch = EpochId(args.epoch);

    println!("== CPUT oracle DON (Layer 1) ==");
    println!(
        "threshold = {}-of-{}\n",
        policy::ORACLE_THRESHOLD,
        policy::ORACLE_SET_SIZE
    );

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

    let intent = submit_report_from_epoch(&report)?;
    println!(
        "sui submit_report intent: epoch={} gflops={} signers={}",
        intent.epoch,
        intent.verified_gflops,
        intent.signer_indices.len()
    );
    if let Some(path) = &args.relay_config {
        let cfg = cput_sui::SuiDeployment::from_file(path)?;
        println!("relay config loaded (dry_run={})", cfg.dry_run);
    }
    Ok(())
}
