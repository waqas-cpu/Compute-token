//! Typed payloads exchanged between pipeline daemons.

use cput_gates::contracts::{AttestationPacket, EpochReport, MintInstruction};
use cput_layer0_compute::TelemetrySample;
use serde::{Deserialize, Serialize};

/// Stable operator-chosen label for a compute node (e.g. `gpu-h100-0`).
pub type NodeKey = String;

/// Hardware profile registered before telemetry polling begins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistration {
    /// Operator label for this node.
    pub node_key: NodeKey,
    /// Hardware class (e.g. `gpu-h100`).
    pub hardware_class: String,
    /// Rated sustained throughput in GFLOP/s.
    pub rated_gflops_per_sec: u128,
    /// Jurisdiction tag.
    pub jurisdiction: String,
    /// TEE enclave measurement.
    pub enclave_measurement: [u8; 32],
    /// Workload hash committed for the epoch.
    pub workload_hash: [u8; 32],
    /// TEE-verified output hash.
    pub verified_hash: [u8; 32],
}

/// One flop-meter poll pushed to the telemetry queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryPoll {
    /// Node that produced this sample.
    pub node_key: NodeKey,
    /// Epoch window the sample belongs to.
    pub epoch: u64,
    /// Zero-based poll index within the epoch.
    pub tick: u64,
    /// Seconds each sample represents.
    pub sample_interval_secs: u64,
    /// Raw telemetry reading.
    pub sample: TelemetrySample,
}

/// Envelope stored in sled with monotonic sequence metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QueueRecord<T> {
    /// Monotonic sequence within the topic.
    pub seq: u64,
    /// Serialised payload.
    pub payload: T,
}

/// Topics in the vertical pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueTopic {
    /// L0 flop-meter → L0 node (`TelemetryPoll`).
    Telemetry,
    /// L0 node → L1 oracle (`AttestationPacket`).
    Attestations,
    /// L1 oracle → L2 agent (`EpochReport`).
    EpochReports,
    /// L2 agent → relayer (`MintInstruction`).
    MintInstructions,
}

impl QueueTopic {
    pub(crate) fn tree_name(self) -> &'static [u8] {
        match self {
            Self::Telemetry => b"q:telemetry",
            Self::Attestations => b"q:attestations",
            Self::EpochReports => b"q:epoch_reports",
            Self::MintInstructions => b"q:mint_instructions",
        }
    }

    pub(crate) fn seq_meta_key(self) -> &'static [u8] {
        match self {
            Self::Telemetry => b"seq:telemetry",
            Self::Attestations => b"seq:attestations",
            Self::EpochReports => b"seq:epoch_reports",
            Self::MintInstructions => b"seq:mint_instructions",
        }
    }
}

/// Re-export gate contracts used as queue payloads.
pub type AttestationEnvelope = AttestationPacket;
/// Epoch report payload on the L1→L2 topic.
pub type EpochReportEnvelope = EpochReport;
/// Mint instruction payload on the L2→relayer topic.
pub type MintInstructionEnvelope = MintInstruction;
