//! Simulated flop-meter daemon.
//!
//! Polls synthetic GPU telemetry every `--interval` seconds and appends
//! [`TelemetryPoll`] records to the pipeline queue. Use `--once` for CI /
//! smoke tests (no sleep, full epoch burst).

use clap::Parser;
use cput_core::ids::{EpochId, WorkloadHash};
use cput_core::policy::EPOCH_SECONDS;
use cput_layer0_compute::{SimulatedFlopMeter, SimulatedFlopMeterConfig};
use cput_queue::{NodeRegistration, PipelineQueue, QueueTopic, TelemetryPoll};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "cput-flop-meter",
    about = "Simulated flop-meter — polls telemetry into the pipeline queue"
)]
struct Args {
    /// Pipeline queue directory (sled).
    #[arg(long, default_value = "data/pipeline")]
    queue: PathBuf,
    /// Epoch being sampled.
    #[arg(long, default_value_t = 1000)]
    epoch: u64,
    /// Poll interval in seconds.
    #[arg(long, default_value_t = 30)]
    interval: u64,
    /// Push one full epoch of samples and exit (no sleep).
    #[arg(long)]
    once: bool,
    /// Run continuously, polling every interval until interrupted.
    #[arg(long)]
    daemon: bool,
}

#[derive(Clone)]
struct NodeProfile {
    node_key: String,
    hardware_class: String,
    rated_gflops_per_sec: u128,
    util_bps: u16,
    thermal_base: u16,
    workload_byte: u8,
}

fn default_profiles() -> Vec<NodeProfile> {
    vec![
        NodeProfile {
            node_key: "gpu-h100-0".into(),
            hardware_class: "gpu-h100".into(),
            rated_gflops_per_sec: 50_000_000,
            util_bps: 8_500,
            thermal_base: 68,
            workload_byte: 0,
        },
        NodeProfile {
            node_key: "gpu-a100-1".into(),
            hardware_class: "gpu-a100".into(),
            rated_gflops_per_sec: 30_000_000,
            util_bps: 8_800,
            thermal_base: 70,
            workload_byte: 1,
        },
        NodeProfile {
            node_key: "tpu-v5-2".into(),
            hardware_class: "tpu-v5".into(),
            rated_gflops_per_sec: 40_000_000,
            util_bps: 9_100,
            thermal_base: 72,
            workload_byte: 2,
        },
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if !args.once && !args.daemon {
        return Err("specify --once (burst) or --daemon (continuous polling)".into());
    }

    let queue = PipelineQueue::open(&args.queue)?;
    let epoch = EpochId(args.epoch);
    let interval = args.interval.max(1);
    let ticks_per_epoch = EPOCH_SECONDS.div_ceil(interval);
    let profiles = default_profiles();

    println!("== cput-flop-meter epoch {} ==", epoch.0);
    println!(
        "queue={} interval={}s ticks/epoch={} mode={}",
        args.queue.display(),
        interval,
        ticks_per_epoch,
        if args.once { "once" } else { "daemon" }
    );

    for profile in &profiles {
        queue.put_node_registration(&NodeRegistration {
            node_key: profile.node_key.clone(),
            hardware_class: profile.hardware_class.clone(),
            rated_gflops_per_sec: profile.rated_gflops_per_sec,
            jurisdiction: "US".into(),
            enclave_measurement: [profile.workload_byte; 32],
            workload_hash: WorkloadHash([profile.workload_byte; 32]).0,
            verified_hash: [0xAB; 32],
        })?;
        println!("registered node {}", profile.node_key);
    }

    let meters: Vec<_> = profiles
        .iter()
        .map(|p| {
            (
                p.node_key.clone(),
                SimulatedFlopMeter::new(SimulatedFlopMeterConfig {
                    sample_interval_secs: interval,
                    rated_gflops_per_sec: p.rated_gflops_per_sec,
                    util_bps: p.util_bps,
                    thermal_c: p.thermal_base,
                    ..Default::default()
                }),
            )
        })
        .collect();

    let mut tick: u64 = 0;
    loop {
        if tick >= ticks_per_epoch {
            println!("epoch {} sampling complete ({} ticks)", epoch.0, tick);
            break;
        }

        for (node_key, meter) in &meters {
            let sample = meter.sample_at(tick);
            let flops_per_sec = sample.flops_per_sec;
            let seq = queue.push_telemetry(&TelemetryPoll {
                node_key: node_key.clone(),
                epoch: epoch.0,
                tick,
                sample_interval_secs: interval,
                sample,
            })?;
            println!("tick={tick} node={node_key} seq={seq} flops/s={flops_per_sec}");
        }

        tick += 1;
        if args.once {
            continue;
        }
        thread::sleep(Duration::from_secs(interval));
    }

    println!(
        "== flop-meter done: {} telemetry records ==",
        queue.head_seq(QueueTopic::Telemetry)?
    );
    Ok(())
}
