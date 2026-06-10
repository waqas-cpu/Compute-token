//! Relayer: submit Sui transactions via CLI and track state.

use crate::adapter::{
    agent_quorum_from_instruction, execute_mint_from_instruction, pqc_digests_from_instruction,
    submit_report_from_epoch,
};
use crate::types::AgentQuorumIntent;
use crate::config::SuiDeployment;
use crate::reconcile::reconcile_epoch_mint;
use crate::rpc::RpcClient;
use crate::types::{ExecuteMintIntent, SubmitReportIntent};
use cput_core::ids::EpochId;
use cput_core::units::Gflops;
use cput_core::{CputError, CputResult};
use cput_gates::contracts::{EpochReport, MintInstruction, SettlementReceipt};
use cput_store::{EpochRecord, RelayerStore, TxRecord, TxStatus};
use std::process::Command;

/// Production relayer bridging off-chain gates to Sui shared objects.
pub struct SuiRelayer {
    cfg: SuiDeployment,
    store: RelayerStore,
    rpc: RpcClient,
}

impl SuiRelayer {
    /// Open the relayer with deployment config and durable store path.
    pub fn open(cfg: SuiDeployment, store_path: impl AsRef<std::path::Path>) -> CputResult<Self> {
        let rpc = RpcClient::new(&cfg)?;
        let store = RelayerStore::open(store_path)?;
        Ok(Self { cfg, store, rpc })
    }

    /// Post an oracle epoch report on-chain (Gate 1→2 mirror).
    pub fn post_epoch_report(&self, report: &EpochReport) -> CputResult<SubmitReportIntent> {
        let intent = submit_report_from_epoch(report)?;
        self.run_call(
            "oracle_verifier",
            "submit_report",
            &[
                &self.cfg.admin_cap,
                &self.cfg.oracle_state,
                &intent.epoch.to_string(),
                &intent.verified_gflops.to_string(),
                &format_hash32(&intent.zk_commitment),
                &format_signer_indices(&intent.signer_indices),
            ],
            EpochId(intent.epoch),
            "submit_report",
        )?;
        Ok(intent)
    }

    /// Post an epoch mint on-chain (Gate 2→4 mirror).
    pub fn post_execute_mint(
        &self,
        instruction: &MintInstruction,
        receipt: &SettlementReceipt,
        verified_gflops: Gflops,
    ) -> CputResult<ExecuteMintIntent> {
        let intent = execute_mint_from_instruction(instruction)?;
        if self.store.mint_posted(EpochId(intent.epoch))? {
            return Err(CputError::Chain(format!(
                "epoch {} mint already posted",
                intent.epoch
            )));
        }
        let body = &receipt.body;
        let record = EpochRecord {
            epoch: intent.epoch,
            receipt: body.clone(),
            txs: vec![],
            reconciled: false,
            reconcile_note: None,
        };
        self.store.put_epoch(&record)?;

        let book_id = self
            .cfg
            .agent_quorum_book
            .as_deref()
            .unwrap_or("0xREPLACE_AGENT_QUORUM_BOOK");
        if let (Some(registry), Some(book)) =
            (&self.cfg.agent_registry, &self.cfg.agent_quorum_book)
        {
            let aq = agent_quorum_from_instruction(instruction)?;
            self.post_agent_quorum(&aq, registry, book)?;
        } else if !self.cfg.dry_run {
            return Err(CputError::Chain(
                "agent_registry and agent_quorum_book required for agent-centric minting".into(),
            ));
        } else {
            eprintln!(
                "[dry-run] agent_registry/agent_quorum_book not configured — skipping approve_epoch_mint"
            );
        }

        self.run_call(
            "minting",
            "execute_mint",
            &[
                &self.cfg.admin_cap,
                &self.cfg.protocol_state,
                &self.cfg.oracle_state,
                book_id,
                &self.cfg.mint_log,
                &intent.epoch.to_string(),
                &intent.total.to_string(),
            ],
            EpochId(intent.epoch),
            "execute_mint",
        )?;

        if !self.cfg.dry_run {
            if let Some(on_chain) = self.rpc.epoch_minted(intent.epoch)? {
                reconcile_epoch_mint(body, &on_chain)?;
                let mut updated = self
                    .store
                    .get_epoch(EpochId(intent.epoch))?
                    .expect("epoch record");
                updated.reconciled = true;
                updated.reconcile_note = Some("EpochMinted event matched receipt".into());
                self.store.put_epoch(&updated)?;
                self.store.set_last_reconciled_epoch(intent.epoch)?;
            }
        }

        if let Some(book) = &self.cfg.pqc_anchor_book {
            if !book.contains("REPLACE") {
                self.post_pqc_anchor(instruction, book)?;
            }
        }

        if let Some(state) = &self.cfg.sponsor.gas_sponsor_state {
            if !state.contains("REPLACE") {
                self.record_sponsored_epoch(intent.epoch, state)?;
            }
        }

        let _ = verified_gflops;
        Ok(intent)
    }

    /// Post AI agent quorum approval on-chain (Gate 2→4 agent-centric mirror).
    pub fn post_agent_quorum(
        &self,
        intent: &AgentQuorumIntent,
        registry_id: &str,
        book_id: &str,
    ) -> CputResult<()> {
        self.run_call(
            "agent_quorum",
            "approve_epoch_mint",
            &[
                &self.cfg.admin_cap,
                registry_id,
                book_id,
                &self.cfg.oracle_state,
                &intent.epoch.to_string(),
                &format_u8_vector(&intent.agent_ids),
                &format_u64_vector(&intent.proposals),
                &intent.total.to_string(),
                &format_hash(&intent.policy_hash),
                &format_hash(&intent.reasoning_trace_hash),
            ],
            EpochId(intent.epoch),
            "approve_agent_quorum",
        )
    }

    /// Record a sponsored epoch in the on-chain gas sponsor registry.
    pub fn record_sponsored_epoch(&self, epoch: u64, state_id: &str) -> CputResult<()> {
        self.run_call(
            "gas_sponsor",
            "record_sponsored_epoch",
            &[state_id, &epoch.to_string()],
            EpochId(epoch),
            "record_sponsored_epoch",
        )
    }

    /// Anchor off-chain-verified PQC envelope digests on-chain.
    pub fn post_pqc_anchor(
        &self,
        instruction: &MintInstruction,
        book_id: &str,
    ) -> CputResult<()> {
        let digests = pqc_digests_from_instruction(instruction);
        let epoch = instruction.signed.body.epoch;
        let epoch_s = epoch.0.to_string();
        let vector_arg = format_digest_vector(&digests);
        self.run_call(
            "pqc_anchor",
            "anchor_epoch",
            &[
                &self.cfg.admin_cap,
                book_id,
                &epoch_s,
                &vector_arg,
            ],
            epoch,
            "anchor_pqc",
        )?;
        Ok(())
    }

    fn run_call(
        &self,
        module: &str,
        function: &str,
        args: &[&str],
        epoch: EpochId,
        kind: &str,
    ) -> CputResult<()> {
        let gas_budget = self.cfg.sponsor.gas_budget.to_string();
        let mut cmd_args = vec![
            "client".to_string(),
            "call".to_string(),
            "--package".to_string(),
            self.cfg.package_id.clone(),
            "--module".to_string(),
            module.to_string(),
            "--function".to_string(),
            function.to_string(),
            "--gas-budget".to_string(),
            gas_budget,
        ];
        if self.cfg.sponsor.active() {
            if let Some(owner) = &self.cfg.sponsor.gas_owner {
                cmd_args.push("--gas-sponsor".to_string());
                cmd_args.push(owner.clone());
            }
        }
        let mut full_args = cmd_args;
        for arg in args {
            full_args.push("--args".to_string());
            full_args.push((*arg).to_string());
        }

        if self.cfg.dry_run {
            eprintln!("[dry-run] {} {}", self.cfg.sui_cli, full_args.join(" "));
            if let Ok(Some(mut rec)) = self.store.get_epoch(epoch) {
                rec.txs.push(TxRecord {
                    kind: kind.to_string(),
                    digest: None,
                    status: TxStatus::Pending,
                    error: None,
                });
                let _ = self.store.put_epoch(&rec);
            }
            return Ok(());
        }

        let output = Command::new(&self.cfg.sui_cli)
            .args(&full_args)
            .output()
            .map_err(|e| CputError::Chain(format!("sui cli spawn: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let digest = extract_digest(&stdout).or_else(|| extract_digest(&stderr));

        let status = if output.status.success() {
            TxStatus::Confirmed
        } else {
            TxStatus::Failed
        };

        if let Ok(Some(mut rec)) = self.store.get_epoch(epoch) {
            rec.txs.push(TxRecord {
                kind: kind.to_string(),
                digest,
                status,
                error: if output.status.success() {
                    None
                } else {
                    Some(format!("{stdout}{stderr}"))
                },
            });
            self.store.put_epoch(&rec)?;
        }

        if !output.status.success() {
            return Err(CputError::Chain(format!(
                "sui call failed: {stdout}{stderr}"
            )));
        }
        Ok(())
    }
}

fn format_u8_vector(values: &[u8]) -> String {
    let inner: Vec<String> = values.iter().map(|v| v.to_string()).collect();
    format!("[{}]", inner.join(","))
}

fn format_u64_vector(values: &[u64]) -> String {
    let inner: Vec<String> = values.iter().map(|v| v.to_string()).collect();
    format!("[{}]", inner.join(","))
}

fn format_hash(hash: &[u8; 32]) -> String {
    format!("0x{}", hex::encode(hash))
}

fn format_hash32(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn format_signer_indices(indices: &[u16]) -> String {
    let inner: Vec<String> = indices.iter().map(|i| i.to_string()).collect();
    format!("[{}]", inner.join(","))
}

fn format_digest_vector(digests: &[[u8; 32]]) -> String {
    let parts: Vec<String> = digests
        .iter()
        .map(|d| format!("0x{}", hex::encode(d)))
        .collect();
    format!("[{}]", parts.join(","))
}

fn extract_digest(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Transaction Digest:") {
            return trimmed.split(':').nth(1).map(|s| s.trim().to_string());
        }
    }
    None
}
