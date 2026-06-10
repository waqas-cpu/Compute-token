//! Minting agent daemon (Layer 2).
//!
//! Three [`MintingAgent`] instances form the minting quorum (R2.2). Each
//! independently applies the committed emission policy; spread must stay within
//! [`CONSENSUS_TOLERANCE_BPS`]. Use `--health` for a readiness probe without
//! minting. Use `--ingest` to consume queued epoch reports.

mod ingest;

use clap::Parser;
use cput_core::ids::{AgentId, EpochId, OracleId, WorkloadHash};
use cput_core::policy;
use cput_core::units::Gflops;
use cput_gates::gate::GateConfig;
use cput_keycustody::{load_mldsa, KeyBackend};
use cput_layer0_compute::ComputeNode;
use cput_layer1_oracle::{OracleNetwork, OracleNode};
use cput_layer2_agentic::{
    assess_quorum_health, default_emission_policy, AgentQuorum, MintingAgent,
};
use cput_pqc::AlgorithmRegistry;
use cput_sui::execute_mint_from_instruction;
use cput_zk::ZkBackend;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "cput-agent", about = "CPUT minting agent quorum (Layer 2)")]
struct Args {
    /// Epoch to process.
    #[arg(long, default_value_t = 7)]
    epoch: u64,
    /// Run quorum health assessment only (readiness probe).
    #[arg(long)]
    health: bool,
    /// Key custody backend: ephemeral | file | tee | hsm.
    #[arg(long, default_value = "ephemeral")]
    key_backend: String,
    /// Path to operator key file (required when key_backend=file).
    #[arg(long)]
    key_file: Option<PathBuf>,
    /// Ingest epoch reports from the pipeline queue and emit mint instructions.
    #[arg(long)]
    ingest: bool,
    /// Pipeline queue directory.
    #[arg(long, default_value = "data/pipeline")]
    queue: PathBuf,
}

fn parse_backend(s: &str) -> KeyBackend {
    match s.to_ascii_lowercase().as_str() {
        "file" => KeyBackend::File,
        "tee" | "tee-sealed" => KeyBackend::TeeSealed,
        "hsm" => KeyBackend::Hsm,
        _ => KeyBackend::Ephemeral,
    }
}

fn build_agents() -> Result<Vec<MintingAgent>, Box<dyn std::error::Error>> {
    (0..policy::AGENT_QUORUM_SIZE as u8)
        .map(|i| MintingAgent::new(AgentId(i)))
        .collect::<Result<_, _>>()
        .map_err(Into::into)
}

fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("cput-agent error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let registry = AlgorithmRegistry::default();
    let backend = ZkBackend::from_env();
    let epoch = EpochId(args.epoch);
    let emission = default_emission_policy();
    let agents = build_agents()?;
    let verified = Gflops(36_000_000);

    println!("== CPUT minting agents (Layer 2) ==");
    println!(
        "quorum = {} MintingAgent instances, tolerance = {} bps",
        policy::AGENT_QUORUM_SIZE,
        policy::CONSENSUS_TOLERANCE_BPS.0
    );
    for agent in &agents {
        println!(
            "  agent {}: kind={}, pubkey={}",
            agent.id.0,
            agent.kind_label(),
            hex::encode(agent.public_key())
        );
    }
    println!(
        "emission policy: {} GFLOP/token, version={}",
        emission.tokens_per_gflop, emission.version
    );

    let health = assess_quorum_health(&agents, &emission, verified)?;
    if args.ingest {
        println!("== cput-agent ingest epoch {} ==", epoch.0);
        let key_backend = parse_backend(&args.key_backend);
        let instruction = ingest::ingest_epoch_reports(
            &args.queue,
            epoch,
            key_backend,
            args.key_file.as_deref(),
        )?;
        let mint_intent = execute_mint_from_instruction(&instruction)?;
        println!(
            "sui execute_mint intent: epoch={} total={}",
            mint_intent.epoch, mint_intent.total
        );
        println!("== agent ingest complete ==");
        return Ok(());
    }
    if args.health {
        let json = serde_json::to_string_pretty(&health)?;
        println!("{json}");
        if !health.healthy {
            return Err("quorum unhealthy".into());
        }
        return Ok(());
    }

    // Full mint path: L0 → L1 → L2.
    let mut attestations = Vec::new();
    for i in 0..3u8 {
        let node = ComputeNode::register(&backend, "gpu-h100", 50_000_000, "EU", [i; 32])?;
        let packet = node.produce_attestation(
            epoch,
            Gflops(12_000_000),
            WorkloadHash([i; 32]),
            70,
            [0xEF; 32],
        )?;
        attestations.push(packet);
    }
    let operators: Vec<OracleNode> = (0..policy::ORACLE_SET_SIZE as u16)
        .map(OracleId)
        .map(OracleNode::new)
        .collect::<Result<_, _>>()?;
    let don = OracleNetwork::new(&registry, &backend, operators);
    let oracle_keys = don.key_set();
    let report = don.finalize_epoch(epoch, &attestations)?;

    let key_backend = parse_backend(&args.key_backend);
    let coordinator = load_mldsa(
        key_backend,
        "agent-coordinator",
        args.key_file.as_deref(),
    )?;

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
    let body = &instruction.signed.body;
    println!("consensus mint total: {} CPUT", body.total.0);
    println!("allocations: {}", body.allocations.len());
    println!("agent proposal signatures: {}", instruction.agent_proposals.len());
    println!(
        "quorum spread: {} bps (healthy={})",
        health.spread_bps, health.healthy
    );

    let mint_intent = execute_mint_from_instruction(&instruction)?;
    println!(
        "sui execute_mint intent: epoch={} total={}",
        mint_intent.epoch, mint_intent.total
    );
    Ok(())
}
