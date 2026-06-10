//! Queue ingest mode: epoch reports → mint instructions.

use cput_core::ids::{AgentId, OracleId};
use cput_core::policy;
use cput_gates::contracts::MintInstruction;
use cput_gates::gate::GateConfig;
use cput_layer2_agentic::{
    assess_quorum_health, default_emission_policy, AgentQuorum, MintingAgent,
};
use cput_keycustody::{load_mldsa, KeyBackend};
use cput_pqc::AlgorithmRegistry;
use cput_queue::{PipelineQueue, QueueTopic};
use cput_zk::ZkBackend;
use std::collections::BTreeMap;
use std::path::Path;

fn oracle_keys_from_report(report: &cput_gates::contracts::EpochReport) -> BTreeMap<OracleId, Vec<u8>> {
    report
        .threshold_sig
        .shares
        .iter()
        .map(|(id, env)| (*id, env.signer_public.clone()))
        .collect()
}

/// Consume queued epoch reports, run the agent quorum, push mint instructions.
pub fn ingest_epoch_reports(
    queue_path: &Path,
    epoch: cput_core::ids::EpochId,
    key_backend: KeyBackend,
    key_file: Option<&Path>,
) -> Result<MintInstruction, Box<dyn std::error::Error>> {
    let queue = PipelineQueue::open(queue_path)?;
    let reports = queue.drain_epoch_reports(epoch)?;
    if reports.is_empty() {
        return Err(format!(
            "no epoch reports in queue for epoch {} — run cput-oracle --ingest first",
            epoch.0
        )
        .into());
    }
    if reports.len() > 1 {
        return Err(format!(
            "multiple epoch reports for epoch {} (expected 1)",
            epoch.0
        )
        .into());
    }
    let report = &reports[0];

    let registry = AlgorithmRegistry::default();
    let backend = ZkBackend::from_env();
    let emission = default_emission_policy();
    let agents: Vec<MintingAgent> = (0..policy::AGENT_QUORUM_SIZE as u8)
        .map(AgentId)
        .map(MintingAgent::new)
        .collect::<Result<_, _>>()?;

    let health = assess_quorum_health(&agents, &emission, report.body.verified_gflops_total)?;
    if !health.healthy {
        return Err(format!(
            "agent quorum unhealthy: spread {} bps",
            health.spread_bps
        )
        .into());
    }

    let oracle_keys = oracle_keys_from_report(report);

    let coordinator = load_mldsa(key_backend, "agent-coordinator", key_file)?;
    let quorum = AgentQuorum::new(
        GateConfig {
            registry: &registry,
            backend: &backend,
        },
        agents,
        coordinator,
        emission,
    );
    let instruction = quorum.mint_for_report(&oracle_keys, report)?;
    let seq = queue.push_mint_instruction(&instruction)?;
    println!(
        "mint instruction seq={seq} total={} proposals={}",
        instruction.signed.body.total.0,
        instruction.agent_proposals.len()
    );
    println!(
        "mint_instructions queue head = {}",
        queue.head_seq(QueueTopic::MintInstructions)?
    );
    Ok(instruction)
}
