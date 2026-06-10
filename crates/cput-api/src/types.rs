//! JSON shapes consumed by the Next.js dashboard.

use serde::{Deserialize, Serialize};

/// Aggregate payload for initial dashboard load.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardPayload {
    /// Hero stat cards.
    pub hero_stats: HeroStats,
    /// Live compute tasks from registered nodes.
    pub tasks: Vec<ComputeTask>,
    /// Relayer / on-chain transaction rows.
    pub transactions: Vec<TransactionRow>,
    /// Pipeline queue depths.
    pub pipeline: PipelineStatus,
    /// API source label (`live` vs `fallback`).
    pub source: String,
}

/// Hero stat cards at the top of the dashboard.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroStats {
    /// User-facing token balance (from confirmed mints).
    pub token_balance: f64,
    /// Token ticker.
    pub token_symbol: String,
    /// Placeholder trend until historical series is wired.
    pub balance_trend_percent: f64,
    /// Nodes with recent telemetry.
    pub active_tasks: u32,
    /// Average VRAM util across latest telemetry samples (0–100).
    pub core_load_percent: u32,
    /// Estimated USD spent at fixed demo rate.
    pub total_spent_usd: f64,
}

/// Single compute task in the live status panel.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeTask {
    /// Stable task id.
    pub id: String,
    /// Human-readable label.
    pub name: String,
    /// Task lifecycle.
    pub status: TaskStatus,
    /// Hardware class from node registration.
    pub gpu_class: String,
    /// Relative start label.
    pub started_at: String,
}

/// Task lifecycle states exposed to the UI.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum TaskStatus {
    /// Actively producing telemetry.
    Running,
    /// Finished epoch work.
    Completed,
    /// Registered but no telemetry yet.
    Queued,
}

/// Row in the transaction history table.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionRow {
    /// Tx digest or placeholder.
    pub hash: String,
    /// Action label.
    pub action: TxAction,
    /// Formatted amount string.
    pub amount: String,
    /// ISO-ish timestamp for display.
    pub timestamp: String,
    /// Settlement status.
    pub status: TxRowStatus,
}

/// Transaction action types shown in the ledger.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum TxAction {
    /// Epoch mint.
    Mint,
    /// Fee burn / compute payment.
    #[serde(rename = "Compute Payment")]
    ComputePayment,
}

/// Ledger row status.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum TxRowStatus {
    /// Confirmed on-chain or reconciled.
    Success,
    /// Submitted / pending.
    Pending,
    /// Failed submission.
    Failed,
}

/// Per-topic queue head sequences.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStatus {
    /// Telemetry poll count.
    pub telemetry: u64,
    /// Attestation packet count.
    pub attestations: u64,
    /// Epoch report count.
    pub epoch_reports: u64,
    /// Mint instruction count.
    pub mint_instructions: u64,
    /// Registered compute nodes.
    pub registered_nodes: u32,
    /// Last reconciled epoch from relayer store (if any).
    pub last_reconciled_epoch: Option<u64>,
}

/// Mint intent submitted from the dashboard.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MintRequest {
    /// Requested cTMP amount.
    pub amount: u64,
}

/// Response after validating a mint request against the pipeline.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MintResponse {
    /// `accepted`, `queued`, or `rejected`.
    pub status: String,
    /// Requested amount.
    pub amount: u64,
    /// Target epoch (latest mint instruction or next cursor).
    pub epoch: u64,
    /// Human-readable detail.
    pub message: String,
    /// Mirrors relayer `dry_run` config.
    pub dry_run: bool,
}

/// Simple health payload.
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    /// Always `ok` when the server is up.
    pub status: &'static str,
    /// API semver.
    pub version: &'static str,
}
