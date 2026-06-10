//! Queue ingest mode: telemetry polls → signed attestation packets.

use cput_core::ids::{EpochId, WorkloadHash};
use cput_core::policy::EPOCH_SECONDS;
use cput_gates::contracts::AttestationPacket;
use cput_layer0_compute::ComputeNode;
use cput_queue::{PipelineQueue, QueueTopic};
use cput_zk::ZkBackend;
use std::path::Path;

/// Consume queued telemetry for `epoch`, sign attestations, push to the
/// attestations topic.
pub fn ingest_telemetry(
    queue_path: &Path,
    epoch: EpochId,
) -> Result<Vec<AttestationPacket>, Box<dyn std::error::Error>> {
    let queue = PipelineQueue::open(queue_path)?;
    let backend = ZkBackend::from_env();
    let grouped = queue.telemetry_by_node(epoch)?;

    if grouped.is_empty() {
        return Err(format!(
            "no telemetry polls in queue for epoch {} — run cput-flop-meter first",
            epoch.0
        )
        .into());
    }

    println!(
        "ingesting {} node(s) from queue {}",
        grouped.len(),
        queue_path.display()
    );

    let mut packets = Vec::new();
    for (node_key, polls) in &grouped {
        let reg = queue
            .get_node_registration(node_key)?
            .ok_or_else(|| format!("missing NodeRegistration for {node_key}"))?;
        let interval = polls
            .first()
            .map(|p| p.sample_interval_secs)
            .unwrap_or(30);
        let expected = EPOCH_SECONDS.div_ceil(interval);
        if polls.len() < expected as usize {
            return Err(format!(
                "node {node_key}: have {} samples, need {expected} for epoch {}",
                polls.len(),
                epoch.0
            )
            .into());
        }

        let node = ComputeNode::register(
            &backend,
            &reg.hardware_class,
            reg.rated_gflops_per_sec,
            &reg.jurisdiction,
            reg.enclave_measurement,
        )?;
        let samples: Vec<_> = polls.iter().map(|p| p.sample.clone()).collect();
        let packet = node.produce_attestation_from_samples(
            epoch,
            &samples,
            interval,
            WorkloadHash(reg.workload_hash),
            reg.verified_hash,
        )?;
        let seq = queue.push_attestation(&packet)?;
        println!(
            "node {node_key} -> attestation seq={seq} gflops={} ({class})",
            packet.signed.body.gflops.0,
            class = reg.hardware_class
        );
        packets.push(packet);
    }

    println!(
        "attestations queue head = {}",
        queue.head_seq(QueueTopic::Attestations)?
    );
    Ok(packets)
}

/// Return true when every registered node has a full epoch of telemetry polls.
#[allow(dead_code)]
pub fn epoch_telemetry_ready(queue_path: &Path, epoch: EpochId) -> Result<bool, Box<dyn std::error::Error>> {
    let queue = PipelineQueue::open(queue_path)?;
    let grouped = queue.telemetry_by_node(epoch)?;
    if grouped.is_empty() {
        return Ok(false);
    }
    for (node_key, polls) in &grouped {
        let interval = polls.first().map(|p| p.sample_interval_secs).unwrap_or(30);
        let expected = EPOCH_SECONDS.div_ceil(interval);
        if polls.len() < expected as usize {
            println!(
                "waiting: {node_key} {}/{} samples",
                polls.len(),
                expected
            );
            return Ok(false);
        }
    }
    Ok(true)
}
