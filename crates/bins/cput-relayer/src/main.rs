//! CPUT Sui relayer — posts verified epoch artifacts on-chain and reconciles.
//!
//! Reads a deployment config (`config/cput.example.toml`), replays a single
//! epoch through the off-chain pipeline (L1→L2→L4→L5), then relays
//! `submit_report` and `execute_mint` to Sui. Use `--dry-run` to validate
//! without submitting transactions.

use clap::Parser;
use cput_core::ids::{AgentId, EpochId, OracleId};
use cput_core::policy;
use cput_core::units::{Gflops, TokenAmount};
use cput_gates::gate::GateConfig;
use cput_layer1_oracle::{OracleNetwork, OracleNode};
use cput_layer2_agentic::{
    assess_quorum_health, default_emission_policy, AgentQuorum, MintingAgent,
};
use cput_layer4_tokenomics::TokenomicsEngine;
use cput_layer5_settlement::SettlementLayer;
use cput_pqc::mldsa::MlDsaKeypair;
use cput_pqc::slhdsa::SlhDsaKeypair;
use cput_pqc::AlgorithmRegistry;
use cput_sui::SuiDeployment;
use cput_sui::SuiRelayer;
use cput_zk::{ProofBackend, ZkBackend};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "cput-relayer", about = "CPUT Sui relayer daemon")]
struct Args {
    /// Path to Sui deployment TOML.
    #[arg(long, default_value = "config/cput.example.toml")]
    config: PathBuf,
    /// Persistent relayer store directory.
    #[arg(long, default_value = "data/relayer")]
    store: PathBuf,
    /// Epoch to process.
    #[arg(long, default_value_t = 1000)]
    epoch: u64,
    /// Force dry-run (overrides config).
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut cfg = SuiDeployment::from_file(&args.config)?;
    if args.dry_run {
        cfg.dry_run = true;
    }

    let registry = AlgorithmRegistry::default();
    let backend = ZkBackend::from_env();
    println!("zk backend = {}", backend);
    let epoch = EpochId(args.epoch);

    println!("== CPUT relayer epoch {} (dry_run={}) ==", epoch.0, cfg.dry_run);

    // Minimal L1 path: synthesise one attestation batch and finalize report.
    let operators: Vec<OracleNode> = (0..policy::ORACLE_SET_SIZE as u16)
        .map(OracleId)
        .map(OracleNode::new)
        .collect::<Result<_, _>>()?;
    let don = OracleNetwork::new(&registry, &backend, operators);
    let oracle_keys = don.key_set();

    // For relayer demo, ingest empty attestations is invalid — use layer0 via node bin in production.
    // Here we require pre-serialized report path or run embedded demo attestations.
    let attestations = demo_attestations(&backend, epoch)?;
    let report = don.finalize_epoch(epoch, &attestations)?;

    let coordinator = MlDsaKeypair::generate()?;
    let coordinator_key = coordinator.public_bytes();
    let agents: Vec<MintingAgent> = (0..policy::AGENT_QUORUM_SIZE as u8)
        .map(AgentId)
        .map(MintingAgent::new)
        .collect::<Result<_, _>>()?;
    let emission = default_emission_policy();
    let agent_health = assess_quorum_health(&agents, &emission, report.body.verified_gflops_total)?;
    if !agent_health.healthy {
        return Err(format!(
            "agent quorum unhealthy: spread {} bps > tolerance {} bps",
            agent_health.spread_bps, agent_health.tolerance_bps
        )
        .into());
    }
    println!(
        "agent quorum healthy: spread={} bps, consensus_total={}",
        agent_health.spread_bps, agent_health.consensus_total
    );
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

    let engine_signer = MlDsaKeypair::generate()?;
    let mut engine = TokenomicsEngine::new(
        GateConfig {
            registry: &registry,
            backend: &backend,
        },
        engine_signer,
        coordinator_key,
        TokenAmount(u64::MAX as u128),
    );
    let receipt = engine.settle(report.body.verified_gflops_total, &instruction)?;

    let governance_key = SlhDsaKeypair::generate()?;
    let settlement = SettlementLayer::new(&registry, governance_key);
    let commitment = settlement.finalize_settlement(&receipt)?;
    println!(
        "off-chain settlement OK; audit root = {}",
        hex::encode(commitment.audit_root)
    );

    let relayer = SuiRelayer::open(cfg, &args.store)?;
    let submit = relayer.post_epoch_report(&report)?;
    println!("submit_report intent: epoch={} gflops={}", submit.epoch, submit.verified_gflops);

    let mint = relayer.post_execute_mint(
        &instruction,
        &receipt,
        report.body.verified_gflops_total,
    )?;
    println!(
        "execute_mint intent: epoch={} total={}",
        mint.epoch, mint.total
    );

    println!("== relayer epoch {} complete ==", epoch.0);
    Ok(())
}

fn demo_attestations<B: ProofBackend>(
    backend: &B,
    epoch: EpochId,
) -> Result<Vec<cput_gates::contracts::AttestationPacket>, Box<dyn std::error::Error>> {
    use cput_core::ids::WorkloadHash;
    use cput_layer0_compute::ComputeNode;
    let mut out = Vec::new();
    for i in 0..3u8 {
        let node = ComputeNode::register(backend, "gpu-h100", 50_000_000, "US", [i; 32])?;
        let packet = node.produce_attestation(
            epoch,
            Gflops(10_000_000 + u128::from(i) * 1_000_000),
            WorkloadHash([i; 32]),
            68,
            [0xAB; 32],
        )?;
        out.push(packet);
    }
    Ok(out)
}
