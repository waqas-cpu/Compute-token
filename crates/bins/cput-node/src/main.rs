//! End-to-end CPUT pipeline demonstration.
//!
//! Drives one full minting cycle through every vertical layer and every
//! integration gate, using the deterministic reference ZK backend:
//!
//! ```text
//! L0 compute ──Gate 0→1──▶ L1 oracle ──Gate 1→2──▶ L2 agents
//!     ──Gate 2→4──▶ L4 tokenomics ──Gate 4→5──▶ L5 settlement
//!     ──downward Gate 5→2/4──▶ governance policy update
//! ```
//!
//! It is a wiring demonstration, not a test: it exercises the producer/consumer
//! path so the whole prototype is shown to compose and run.

use cput_core::ids::{AgentId, EpochId, OracleId, WorkloadHash};
use cput_core::units::{Bps, Gflops, TokenAmount};
use cput_gates::gate::{admit_policy_update, GateConfig};
use cput_layer0_compute::ComputeNode;
use cput_layer1_oracle::{OracleNetwork, OracleNode};
use cput_layer2_agentic::{AgentQuorum, EmissionPolicy, MintingAgent};
use cput_layer4_tokenomics::TokenomicsEngine;
use cput_layer5_settlement::{Ballot, Proposal, SettlementLayer};
use cput_pqc::mldsa::MlDsaKeypair;
use cput_pqc::slhdsa::SlhDsaKeypair;
use cput_pqc::AlgorithmRegistry;
use cput_zk::ReferenceBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = AlgorithmRegistry::default();
    let backend = ReferenceBackend;
    let epoch = EpochId(1_000);

    println!("== CPUT end-to-end pipeline (reference ZK backend) ==");
    println!("epoch = {epoch}\n");

    // ---- Layer 0: register nodes and produce signed, ZK-proven attestations.
    let node_specs = [
        ("gpu-h100", 50_000_000u128),
        ("gpu-a100", 30_000_000),
        ("tpu-v5", 40_000_000),
    ];
    let mut attestations = Vec::new();
    for (i, (class, rate)) in node_specs.iter().enumerate() {
        let node = ComputeNode::register(&backend, *class, *rate, "US", [i as u8; 32])?;
        let gflops = Gflops(rate * 300); // 300s of the 360s epoch at rated speed
        let packet =
            node.produce_attestation(epoch, gflops, WorkloadHash([i as u8; 32]), 72, [0xAB; 32])?;
        println!(
            "L0  node {} attested {} GFLOPs ({class})",
            node.node_id(),
            gflops.0
        );
        attestations.push(packet);
    }

    // ---- Layer 1: oracle DON re-verifies (Gate 0→1) and emits a report.
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

    // ---- Layer 2: agent quorum consumes report (Gate 1→2) and mints.
    let emission = EmissionPolicy {
        tokens_per_gflop: 1,
        version: "v1".into(),
    };
    let coordinator = MlDsaKeypair::generate()?;
    let coordinator_key = coordinator.public_bytes();
    let agents = (0..cput_core::policy::AGENT_QUORUM_SIZE as u8)
        .map(|i| MintingAgent::new(AgentId(i)))
        .collect();
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

    // ---- Layer 4: tokenomics engine settles (Gate 2→4) and splits supply.
    let engine_signer = MlDsaKeypair::generate()?;
    let mut engine = TokenomicsEngine::new(
        GateConfig {
            registry: &registry,
            backend: &backend,
        },
        engine_signer,
        coordinator_key,
        TokenAmount(u128::MAX), // generous genesis ceiling
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

    // ---- Layer 5: settlement finalisation (Gate 4→5) + governance.
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

    // ---- Downward Gate 5→2/4: admit the policy update back into minting.
    admit_policy_update(&registry, &settlement.governance_key(), &policy_update)?;
    engine.set_ceiling(policy_update.body.mint_ceiling);
    println!(
        "↺   downward gate admitted policy update; new ceiling = {}",
        engine.mint_ceiling.0
    );

    // Demonstrate the elastic governor recommendation under low utilisation.
    let recommended = engine.recommend_ceiling(Bps(5_000));
    println!(
        "L4  governor (util 50%) recommends ceiling = {}",
        recommended.0
    );

    println!("\n== pipeline complete: every gate admitted ==");
    Ok(())
}
