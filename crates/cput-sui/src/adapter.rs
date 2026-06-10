//! Off-chain artifact → Sui transaction intent adapters.

use crate::types::{AgentQuorumIntent, ExecuteMintIntent, SetCeilingIntent, SubmitReportIntent};
use cput_core::units::{Gflops, TokenAmount};
use cput_core::{CputError, CputResult};
use cput_gates::contracts::{EpochReport, MintInstruction, PolicyUpdateBody, SettlementReceiptBody};
use cput_core::policy;
use cput_pqc::envelope_digest;
use std::collections::BTreeSet;

/// Build `submit_report` intent from a threshold-signed epoch report.
pub fn submit_report_from_epoch(report: &EpochReport) -> CputResult<SubmitReportIntent> {
    let epoch = report.body.epoch.0;
    let verified_gflops = report.body.verified_gflops_total.to_chain_u64()?;
    let mut indices = BTreeSet::new();
    for (oracle_id, _) in &report.threshold_sig.shares {
        let idx = oracle_id.0;
        if u64::from(idx) >= policy::ORACLE_SET_SIZE as u64 {
            return Err(CputError::Chain(format!(
                "oracle index {idx} out of range (set size {})",
                policy::ORACLE_SET_SIZE
            )));
        }
        indices.insert(idx);
    }
    if indices.len() < policy::ORACLE_THRESHOLD {
        return Err(CputError::ThresholdNotMet {
            have: indices.len(),
            need: policy::ORACLE_THRESHOLD,
        });
    }
    Ok(SubmitReportIntent {
        epoch,
        verified_gflops,
        zk_commitment: report.body.zk_proof_commitment.to_vec(),
        signer_indices: indices.into_iter().collect(),
    })
}

/// SHA-256 envelope digests for on-chain `pqc_anchor::anchor_epoch`.
/// Order: coordinator mint instruction, then each agent proposal (R2.2).
pub fn pqc_digests_from_instruction(instr: &MintInstruction) -> Vec<[u8; 32]> {
    let mut digests = vec![envelope_digest(&instr.signed.envelope)];
    for proposal in &instr.agent_proposals {
        digests.push(envelope_digest(&proposal.envelope));
    }
    digests
}

/// Build `agent_quorum::approve_epoch_mint` intent from a mint instruction.
pub fn agent_quorum_from_instruction(instr: &MintInstruction) -> CputResult<AgentQuorumIntent> {
    if instr.agent_proposals.len() < policy::AGENT_QUORUM_SIZE {
        return Err(CputError::ThresholdNotMet {
            have: instr.agent_proposals.len(),
            need: policy::AGENT_QUORUM_SIZE,
        });
    }
    let body = &instr.signed.body;
    let mut agent_ids = Vec::with_capacity(instr.agent_proposals.len());
    let mut proposals = Vec::with_capacity(instr.agent_proposals.len());
    for p in &instr.agent_proposals {
        let pb = &p.body;
        if pb.epoch != body.epoch {
            return Err(CputError::Chain(format!(
                "agent {} proposal epoch {} != instruction epoch {}",
                pb.agent_id.0, pb.epoch.0, body.epoch.0
            )));
        }
        if pb.policy_hash != body.policy_hash {
            return Err(CputError::Chain(format!(
                "agent {} policy_hash mismatch",
                pb.agent_id.0
            )));
        }
        agent_ids.push(pb.agent_id.0);
        proposals.push(
            u64::try_from(pb.proposal)
                .map_err(|_| CputError::Chain("agent proposal overflows u64".into()))?,
        );
    }
    Ok(AgentQuorumIntent {
        epoch: body.epoch.0,
        agent_ids,
        proposals,
        total: body.total.to_chain_u64()?,
        policy_hash: body.policy_hash,
        reasoning_trace_hash: body.reasoning_trace_hash,
    })
}

/// Build `execute_mint` intent from a mint instruction.
pub fn execute_mint_from_instruction(instr: &MintInstruction) -> CputResult<ExecuteMintIntent> {
    Ok(ExecuteMintIntent {
        epoch: instr.signed.body.epoch.0,
        total: instr.signed.body.total.to_chain_u64()?,
    })
}

/// Build `set_ceiling` intent from a governance policy update body.
pub fn set_ceiling_from_policy(body: &PolicyUpdateBody) -> CputResult<SetCeilingIntent> {
    Ok(SetCeilingIntent {
        mint_ceiling: body.mint_ceiling.to_chain_u64()?,
        circuit_breaker_engaged: body.circuit_breaker_engaged,
    })
}

/// Expected on-chain split from an off-chain settlement receipt (Gate 4→5 mirror).
pub fn expected_epoch_minted(
    receipt: &SettlementReceiptBody,
    verified_gflops: Gflops,
) -> CputResult<crate::types::OnChainEpochMinted> {
    Ok(crate::types::OnChainEpochMinted {
        epoch: receipt.epoch.0,
        total: receipt.minted_total.to_chain_u64()?,
        providers: receipt.providers.to_chain_u64()?,
        oracle: receipt.oracle.to_chain_u64()?,
        treasury: receipt.treasury.to_chain_u64()?,
        burn_reserve: receipt.burn_reserve.to_chain_u64()?,
        verified_gflops: verified_gflops.to_chain_u64()?,
    })
}

/// Expected fee routing from off-chain L4 `FeeRouting` mirror.
pub fn expected_fee_routed(
    fee: TokenAmount,
    burned: TokenAmount,
    providers: TokenAmount,
    treasury: TokenAmount,
) -> CputResult<(u64, u64, u64, u64)> {
    Ok((
        fee.to_chain_u64()?,
        burned.to_chain_u64()?,
        providers.to_chain_u64()?,
        treasury.to_chain_u64()?,
    ))
}
