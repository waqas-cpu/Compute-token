//! Minting agent daemon demonstration (Layer 2).
//!
//! Builds a verified epoch report (L0→L1), then runs the multi-agent minting
//! quorum: each agent independently applies the committed emission policy, the
//! quorum checks the proposals agree within tolerance, and a coordinating agent
//! emits the ML-DSA-signed, ZK-proven mint instruction consumed by Layer 4.

use cput_core::ids::{AgentId, EpochId, OracleId, WorkloadHash};
use cput_core::policy;
use cput_core::units::Gflops;
use cput_gates::gate::GateConfig;
use cput_layer0_compute::ComputeNode;
use cput_layer1_oracle::{OracleNetwork, OracleNode};
use cput_layer2_agentic::{AgentQuorum, EmissionPolicy, MintingAgent};
use cput_pqc::mldsa::MlDsaKeypair;
use cput_pqc::AlgorithmRegistry;
use cput_zk::ReferenceBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = AlgorithmRegistry::default();
    let backend = ReferenceBackend;
    let epoch = EpochId(7);

    println!("== CPUT minting agents (Layer 2) ==");
    println!(
        "quorum = {} agents, consensus tolerance = {} bps\n",
        policy::AGENT_QUORUM_SIZE,
        policy::CONSENSUS_TOLERANCE_BPS.0
    );

    // Build a verified epoch report (L0 → L1) to feed the agents.
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

    // Run the minting quorum (Gate 1→2 on the way in).
    let emission = EmissionPolicy {
        tokens_per_gflop: 2,
        version: "v1".into(),
    };
    println!(
        "emission policy hash: {}",
        hex::encode(emission.policy_hash())
    );
    let coordinator = MlDsaKeypair::generate()?;
    let agents = (0..policy::AGENT_QUORUM_SIZE as u8)
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
    let body = &instruction.signed.body;
    println!("consensus mint total: {} CPUT", body.total.0);
    println!("allocations: {}", body.allocations.len());
    println!(
        "reasoning proof commitment: {}",
        hex::encode(instruction.reasoning_proof.public_inputs_commitment)
    );
    println!(
        "policy hash bound into instruction: {}",
        hex::encode(body.policy_hash)
    );
    Ok(())
}
