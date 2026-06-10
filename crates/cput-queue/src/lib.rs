//! # cput-queue — durable inter-layer pipeline queue
//!
//! Backing store for distributed daemons: flop-meter → node → oracle → agent →
//! relayer. Each topic is an append-only sled tree keyed by monotonic sequence.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod messages;

pub use messages::{
    AttestationEnvelope, EpochReportEnvelope, MintInstructionEnvelope, NodeKey,
    NodeRegistration, QueueTopic, TelemetryPoll,
};

use cput_core::ids::EpochId;
use cput_core::{CputError, CputResult};
use cput_gates::contracts::{AttestationPacket, EpochReport, MintInstruction};
// EpochReport used by drain_epoch_reports
use messages::QueueRecord;
use std::collections::BTreeMap;
use std::path::Path;

const TREE_NODES: &[u8] = b"meta:nodes";
const TREE_SEQ: &[u8] = b"meta:seq";

/// Append-only durable queue for pipeline daemons.
pub struct PipelineQueue {
    db: sled::Db,
}

impl PipelineQueue {
    /// Open (or create) a queue at `path`.
    pub fn open(path: impl AsRef<Path>) -> CputResult<Self> {
        let db = sled::open(path).map_err(|e| CputError::Store(e.to_string()))?;
        Ok(Self { db })
    }

    /// Register a node's hardware profile (idempotent overwrite).
    pub fn put_node_registration(&self, reg: &NodeRegistration) -> CputResult<()> {
        let tree = self
            .db
            .open_tree(TREE_NODES)
            .map_err(|e| CputError::Store(e.to_string()))?;
        tree.insert(
            reg.node_key.as_bytes(),
            serde_json::to_vec(reg).map_err(|e| CputError::Codec(e.to_string()))?,
        )
        .map_err(|e| CputError::Store(e.to_string()))?;
        tree.flush().map_err(|e| CputError::Store(e.to_string()))?;
        Ok(())
    }

    /// Load a node registration by key.
    pub fn get_node_registration(&self, node_key: &str) -> CputResult<Option<NodeRegistration>> {
        let tree = self
            .db
            .open_tree(TREE_NODES)
            .map_err(|e| CputError::Store(e.to_string()))?;
        Ok(tree
            .get(node_key.as_bytes())
            .map_err(|e| CputError::Store(e.to_string()))?
            .map(|v| serde_json::from_slice(&v))
            .transpose()
            .map_err(|e| CputError::Codec(e.to_string()))?)
    }

    /// List all registered node keys.
    pub fn list_node_keys(&self) -> CputResult<Vec<NodeKey>> {
        let tree = self
            .db
            .open_tree(TREE_NODES)
            .map_err(|e| CputError::Store(e.to_string()))?;
        let mut keys = Vec::new();
        for item in tree.iter() {
            let (k, _) = item.map_err(|e| CputError::Store(e.to_string()))?;
            keys.push(String::from_utf8_lossy(&k).into_owned());
        }
        keys.sort();
        Ok(keys)
    }

    /// Append a telemetry poll; returns the assigned sequence number.
    pub fn push_telemetry(&self, poll: &TelemetryPoll) -> CputResult<u64> {
        self.push(QueueTopic::Telemetry, poll)
    }

    /// Append a signed attestation packet.
    pub fn push_attestation(&self, packet: &AttestationPacket) -> CputResult<u64> {
        self.push(QueueTopic::Attestations, packet)
    }

    /// Append a threshold-signed epoch report.
    pub fn push_epoch_report(&self, report: &EpochReport) -> CputResult<u64> {
        self.push(QueueTopic::EpochReports, report)
    }

    /// Append a mint instruction.
    pub fn push_mint_instruction(&self, instruction: &MintInstruction) -> CputResult<u64> {
        self.push(QueueTopic::MintInstructions, instruction)
    }

    /// Drain telemetry polls for `epoch` with sequence strictly greater than `after_seq`.
    pub fn drain_telemetry(
        &self,
        epoch: EpochId,
        after_seq: u64,
        limit: usize,
    ) -> CputResult<Vec<(u64, TelemetryPoll)>> {
        self.drain_filtered(QueueTopic::Telemetry, after_seq, limit, |poll: &TelemetryPoll| {
            poll.epoch == epoch.0
        })
    }

    /// Group telemetry polls for an epoch by node key.
    pub fn telemetry_by_node(
        &self,
        epoch: EpochId,
    ) -> CputResult<BTreeMap<NodeKey, Vec<TelemetryPoll>>> {
        let rows = self.drain_telemetry(epoch, 0, usize::MAX)?;
        let mut grouped: BTreeMap<NodeKey, Vec<TelemetryPoll>> = BTreeMap::new();
        for (_, poll) in rows {
            grouped.entry(poll.node_key.clone()).or_default().push(poll);
        }
        for samples in grouped.values_mut() {
            samples.sort_by_key(|p| p.tick);
        }
        Ok(grouped)
    }

    /// Drain epoch reports for `epoch`.
    pub fn drain_epoch_reports(&self, epoch: EpochId) -> CputResult<Vec<EpochReport>> {
        let rows = self.drain_filtered::<EpochReport, _>(
            QueueTopic::EpochReports,
            0,
            usize::MAX,
            |report: &EpochReport| report.body.epoch == epoch,
        )?;
        Ok(rows.into_iter().map(|(_, r)| r).collect())
    }

    /// Drain mint instructions for `epoch`.
    pub fn drain_mint_instructions(&self, epoch: EpochId) -> CputResult<Vec<MintInstruction>> {
        let rows = self.drain_filtered::<MintInstruction, _>(
            QueueTopic::MintInstructions,
            0,
            usize::MAX,
            |inst: &MintInstruction| inst.signed.body.epoch == epoch,
        )?;
        Ok(rows.into_iter().map(|(_, m)| m).collect())
    }

    /// Drain the most recent mint instructions across all epochs (newest last).
    pub fn drain_recent_mint_instructions(
        &self,
        limit: usize,
    ) -> CputResult<Vec<(u64, MintInstruction)>> {
        self.drain_filtered(QueueTopic::MintInstructions, 0, limit, |_| true)
    }

    /// Drain attestation packets for `epoch`.
    pub fn drain_attestations(&self, epoch: EpochId) -> CputResult<Vec<AttestationPacket>> {
        let rows = self.drain_filtered::<AttestationPacket, _>(
            QueueTopic::Attestations,
            0,
            usize::MAX,
            |pkt: &AttestationPacket| pkt.signed.body.epoch == epoch,
        )?;
        Ok(rows.into_iter().map(|(_, p)| p).collect())
    }

    /// Latest assigned sequence for a topic (0 if empty).
    pub fn head_seq(&self, topic: QueueTopic) -> CputResult<u64> {
        let meta = self
            .db
            .open_tree(TREE_SEQ)
            .map_err(|e| CputError::Store(e.to_string()))?;
        Ok(meta
            .get(topic.seq_meta_key())
            .map_err(|e| CputError::Store(e.to_string()))?
            .map(|v| meta_seq_from_bytes(&v))
            .transpose()?
            .unwrap_or(0))
    }

    fn push<T: serde::Serialize>(&self, topic: QueueTopic, payload: &T) -> CputResult<u64> {
        let seq = self.bump_seq(topic)?;
        let record = QueueRecord {
            seq,
            payload,
        };
        let tree = self
            .db
            .open_tree(topic.tree_name())
            .map_err(|e| CputError::Store(e.to_string()))?;
        tree.insert(
            seq.to_be_bytes(),
            serde_json::to_vec(&record).map_err(|e| CputError::Codec(e.to_string()))?,
        )
        .map_err(|e| CputError::Store(e.to_string()))?;
        tree.flush().map_err(|e| CputError::Store(e.to_string()))?;
        Ok(seq)
    }

    fn bump_seq(&self, topic: QueueTopic) -> CputResult<u64> {
        let meta = self
            .db
            .open_tree(TREE_SEQ)
            .map_err(|e| CputError::Store(e.to_string()))?;
        let next = self.head_seq(topic)? + 1;
        meta.insert(topic.seq_meta_key(), next.to_be_bytes().as_slice())
            .map_err(|e| CputError::Store(e.to_string()))?;
        meta.flush().map_err(|e| CputError::Store(e.to_string()))?;
        Ok(next)
    }

    fn drain_filtered<T, F>(
        &self,
        topic: QueueTopic,
        after_seq: u64,
        limit: usize,
        mut keep: F,
    ) -> CputResult<Vec<(u64, T)>>
    where
        T: serde::de::DeserializeOwned,
        F: FnMut(&T) -> bool,
    {
        let tree = self
            .db
            .open_tree(topic.tree_name())
            .map_err(|e| CputError::Store(e.to_string()))?;
        let start = if after_seq == 0 {
            [0u8; 8]
        } else {
            (after_seq + 1).to_be_bytes()
        };
        let mut out = Vec::new();
        for item in tree.range(start..) {
            let (k, v) = item.map_err(|e| CputError::Store(e.to_string()))?;
            let seq = seq_from_bytes(&k)?;
            let record: QueueRecord<T> =
                serde_json::from_slice(&v).map_err(|e| CputError::Codec(e.to_string()))?;
            if keep(&record.payload) {
                out.push((seq, record.payload));
                if out.len() >= limit {
                    break;
                }
            }
        }
        Ok(out)
    }
}

fn seq_from_bytes(k: &[u8]) -> CputResult<u64> {
    let arr: [u8; 8] = k
        .get(..8)
        .ok_or_else(|| CputError::Store("invalid queue key".into()))?
        .try_into()
        .map_err(|_| CputError::Store("invalid queue key width".into()))?;
    Ok(u64::from_be_bytes(arr))
}

fn meta_seq_from_bytes(v: &[u8]) -> CputResult<u64> {
    let arr: [u8; 8] = v
        .get(..8)
        .ok_or_else(|| CputError::Store("invalid meta seq".into()))?
        .try_into()
        .map_err(|_| CputError::Store("invalid meta seq width".into()))?;
    Ok(u64::from_be_bytes(arr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cput_layer0_compute::TelemetrySample;

    #[test]
    fn telemetry_round_trip() {
        let dir = std::env::temp_dir().join(format!("cput-queue-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let q = PipelineQueue::open(&dir).unwrap();
        let poll = TelemetryPoll {
            node_key: "gpu-0".into(),
            epoch: 42,
            tick: 0,
            sample_interval_secs: 30,
            sample: TelemetrySample {
                flops_per_sec: 1_000_000,
                vram_util_bps: 5000,
                thermal_c: 70,
                power_w: 300,
            },
        };
        q.push_telemetry(&poll).unwrap();
        let rows = q.drain_telemetry(EpochId(42), 0, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1.node_key, "gpu-0");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
