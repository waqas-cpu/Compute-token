//! CPUT compute node (Layer 0).
//!
//! - **Default:** full L0→L5 wiring demonstration (in-process).
//! - **`--ingest`:** read flop-meter telemetry from the pipeline queue, sign
//!   attestations, and push to the attestations topic for `cput-oracle`.

mod ingest;

use clap::Parser;
use cput_core::ids::{AgentId, EpochId, OracleId, WorkloadHash};
use cput_core::units::{Bps, TokenAmount};
use cput_gates::gate::{admit_policy_update, GateConfig};
use cput_layer0_compute::{ComputeNode, SimulatedFlopMeter, SimulatedFlopMeterConfig};
use cput_layer1_oracle::{OracleNetwork, OracleNode};
use cput_layer2_agentic::{default_emission_policy, AgentQuorum, MintingAgent};
use cput_layer4_tokenomics::TokenomicsEngine;
use cput_layer5_settlement::{Ballot, Proposal, SettlementLayer};
use cput_pqc::mldsa::MlDsaKeypair;
use cput_pqc::slhdsa::SlhDsaKeypair;
use cput_pqc::AlgorithmRegistry;
use cput_zk::ZkBackend;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "cput-node", about = "CPUT compute node (Layer 0)")]
struct Args {
    /// Ingest telemetry from the pipeline queue and emit attestations.
    #[arg(long)]
    ingest: bool,
    /// Pipeline queue directory.
    #[arg(long, default_value = "data/pipeline")]
    queue: PathBuf,
    /// Epoch to process (ingest mode).
    #[arg(long, default_value_t = 1000)]
    epoch: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.ingest {
        return run_ingest(&args);
    }
    run_demo()
}

fn run_ingest(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let epoch = EpochId(args.epoch);
    println!("== cput-node ingest epoch {} ==", epoch.0);
    let packets = ingest::ingest_telemetry(&args.queue, epoch)?;
    println!("== ingest complete: {} attestation(s) queued ==", packets.len());
    Ok(())
}

fn run_demo() -> Result<(), Box<dyn std::error::Error>> {
    let registry = AlgorithmRegistry::default();
    let backend = ZkBackend::from_env();
    println!("zk backend = {}", backend);
    let epoch = EpochId(1_000);

    println!("== CPUT end-to-end pipeline (ZK: {}) ==", backend.kind());
    println!("epoch = {epoch}\n");

    let node_specs = [
        ("gpu-h100", 50_000_000u128),
        ("gpu-a100", 30_000_000),
        ("tpu-v5", 40_000_000),
    ];
    let mut attestations = Vec::new();
    for (i, (class, rate)) in node_specs.iter().enumerate() {
        let node = ComputeNode::register(&backend, *class, *rate, "US", [i as u8; 32])?;
        let meter = SimulatedFlopMeter::new(SimulatedFlopMeterConfig {
            rated_gflops_per_sec: *rate,
            util_bps: 8_500 + u16::try_from(i).unwrap_or(0) * 300,
            thermal_c: 68 + u16::try_from(i).unwrap_or(0) * 2,
            ..Default::default()
        });
        let samples = meter.collect_epoch_samples();
        let packet = node.produce_attestation_from_samples(
            epoch,
            &samples,
            meter.sample_interval_secs(),
            WorkloadHash([i as u8; 32]),
            [0xAB; 32],
        )?;
        println!(
            "L0  node {} attested {} GFLOPs from {} flop-meter samples ({class})",
            node.node_id(),
            packet.signed.body.gflops.0,
            samples.len()
        );
        attestations.push(packet);
    }

    let mut operators = Vec::new();
    for i in 0..cput_core::policy::ORACLE_SET_SIZE as u16 {
        operators.push(OracleNode::new(OracleId(i))?);
    }
    let don = OracleNetwork::new(&registry, &backend, operators);
    let oracle_keys = don.key_set();
    let report = don.finalize_epoch(epoch, &attestations)?;
    println!(
        "\nL1  epoch report: {} GFLOPs verified across {} nodes, {} oracle signatures",
        report.body.verified_gflops_total.0,
        report.body.node_scores.len(),
        report.threshold_sig.shares.len()
    );

    let emission = default_emission_policy();
    let coordinator = MlDsaKeypair::generate()?;
    let coordinator_key = coordinator.public_bytes();
    let agents: Vec<MintingAgent> = (0..cput_core::policy::AGENT_QUORUM_SIZE as u8)
        .map(|i| MintingAgent::new(AgentId(i)))
        .collect::<Result<_, _>>()?;
    let quorum = AgentQuorum::new(
        GateConfig {
            registry: &registry,
            backend: &backend,
        },
        agents,
        coordinator,
        emission,
    );
    let instruction = quorum.mint_for_report(&oracle_keys, &report)?;
    println!(
        "L2  mint instruction: total = {} CPUT across {} allocations",
        instruction.signed.body.total.0,
        instruction.signed.body.allocations.len()
    );

    let engine_signer = MlDsaKeypair::generate()?;
    let mut engine = TokenomicsEngine::new(
        GateConfig {
            registry: &registry,
            backend: &backend,
        },
        engine_signer,
        coordinator_key,
        TokenAmount(u128::MAX),
    );
    let receipt = engine.settle(report.body.verified_gflops_total, &instruction)?;
    let b = &receipt.body;
    println!(
        "L4  settlement split: providers={} oracle={} treasury={} burn={} (total={})",
        b.providers.0, b.oracle.0, b.treasury.0, b.burn_reserve.0, b.minted_total.0
    );
    let routing = engine.process_access_fee(epoch, TokenAmount(1_000_000))?;
    println!(
        "L4  burn market: burned={} providers={} treasury={}",
        routing.burned.0, routing.providers.0, routing.treasury.0
    );

    let governance_key = SlhDsaKeypair::generate()?;
    let settlement = SettlementLayer::new(&registry, governance_key);
    let commitment = settlement.finalize_settlement(&receipt)?;
    println!(
        "L5  epoch committed; audit root = {}",
        hex::encode(commitment.audit_root)
    );

    let proposal = Proposal {
        id: 1,
        effective_epoch: epoch.next(),
        mint_ceiling: TokenAmount(900_000_000),
        circuit_breaker_engaged: false,
        approved_algorithms: vec![],
    };
    let ballots = [
        Ballot {
            token_weight: 1_000_000,
            support: true,
        },
        Ballot {
            token_weight: 250_000,
            support: true,
        },
        Ballot {
            token_weight: 400_000,
            support: false,
        },
    ];
    let tally = settlement.tally(&ballots);
    let policy_update = settlement.enact(&proposal, tally)?;
    println!(
        "L5  governance: for={} against={} -> ceiling {}",
        tally.for_power, tally.against_power, proposal.mint_ceiling.0
    );

    admit_policy_update(&registry, &settlement.governance_key(), &policy_update)?;
    engine.set_ceiling(policy_update.body.mint_ceiling);
    println!(
        "↺   downward gate admitted policy update; new ceiling = {}",
        engine.mint_ceiling.0
    );

    let recommended = engine.recommend_ceiling(Bps(5_000));
    println!(
        "L4  governor (util 50%) recommends ceiling = {}",
        recommended.0
    );

    println!("\n== pipeline complete: every gate admitted ==");
    Ok(())
}
