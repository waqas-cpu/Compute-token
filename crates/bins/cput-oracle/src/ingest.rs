//! Queue ingest mode: attestation packets → threshold-signed epoch report.

use cput_core::ids::EpochId;
use cput_core::policy;
use cput_gates::contracts::EpochReport;
use cput_layer1_oracle::{OracleNetwork, OracleNode};
use cput_pqc::AlgorithmRegistry;
use cput_queue::{PipelineQueue, QueueTopic};
use cput_zk::ZkBackend;
use std::path::Path;

/// Consume queued attestations for `epoch`, finalize the DON report, push to
/// the epoch-reports topic.
pub fn ingest_attestations(
    queue_path: &Path,
    epoch: EpochId,
) -> Result<EpochReport, Box<dyn std::error::Error>> {
    let queue = PipelineQueue::open(queue_path)?;
    let attestations = queue.drain_attestations(epoch)?;
    if attestations.is_empty() {
        return Err(format!(
            "no attestations in queue for epoch {} — run cput-node --ingest first",
            epoch.0
        )
        .into());
    }

    println!(
        "ingesting {} attestation(s) from queue {}",
        attestations.len(),
        queue_path.display()
    );

    let registry = AlgorithmRegistry::default();
    let backend = ZkBackend::from_env();
    let operators: Vec<OracleNode> = (0..policy::ORACLE_SET_SIZE as u16)
        .map(cput_core::ids::OracleId)
        .map(OracleNode::new)
        .collect::<Result<_, _>>()?;
    let don = OracleNetwork::new(&registry, &backend, operators);
    let report = don.finalize_epoch(epoch, &attestations)?;
    let seq = queue.push_epoch_report(&report)?;
    println!(
        "epoch report seq={seq} verified_gflops={} signers={}",
        report.body.verified_gflops_total.0,
        report.threshold_sig.shares.len()
    );
    println!(
        "epoch_reports queue head = {}",
        queue.head_seq(QueueTopic::EpochReports)?
    );
    Ok(report)
}
