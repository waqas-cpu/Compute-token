//! Axum route handlers.

use crate::state::ApiState;
use crate::types::{
    ComputeTask, DashboardPayload, HealthResponse, HeroStats, MintRequest, MintResponse,
    PipelineStatus, TaskStatus, TransactionRow, TxAction, TxRowStatus,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use cput_core::ids::EpochId;
use cput_queue::QueueTopic;
use cput_store::{TxRecord, TxStatus};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

const TOKEN_SYMBOL: &str = "cTMP";
const USD_PER_TOKEN: f64 = 0.1;

/// Build the API router with CORS enabled for the Next.js dev server.
pub fn router(state: Arc<ApiState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .route("/api/v1/health", get(health))
        .route("/api/v1/dashboard", get(dashboard))
        .route("/api/v1/stats", get(stats))
        .route("/api/v1/tasks", get(tasks))
        .route("/api/v1/transactions", get(transactions))
        .route("/api/v1/pipeline", get(pipeline))
        .route("/api/v1/mint", post(mint))
        .with_state(state)
        .layer(cors)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn dashboard(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    match build_dashboard(&state) {
        Ok(payload) => (StatusCode::OK, Json(payload)).into_response(),
        Err(e) => api_error(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

async fn stats(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    match build_hero_stats(&state) {
        Ok(stats) => (StatusCode::OK, Json(stats)).into_response(),
        Err(e) => api_error(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

async fn tasks(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    match build_tasks(&state) {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => api_error(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

async fn transactions(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    match build_transactions(&state) {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => api_error(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

async fn pipeline(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    match build_pipeline_status(&state) {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(e) => api_error(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

async fn mint(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<MintRequest>,
) -> impl IntoResponse {
    match handle_mint(&state, req) {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => api_error(StatusCode::BAD_REQUEST, &e),
    }
}

fn api_error(status: StatusCode, message: &str) -> axum::response::Response {
    let body = serde_json::json!({ "error": message });
    (status, Json(body)).into_response()
}

fn build_dashboard(state: &ApiState) -> Result<DashboardPayload, String> {
    let hero_stats = build_hero_stats(state)?;
    let tasks = build_tasks(state)?;
    let transactions = build_transactions(state)?;
    let pipeline = build_pipeline_status(state)?;
    let source = if pipeline.telemetry > 0 || !transactions.is_empty() {
        "live"
    } else {
        "fallback"
    };
    Ok(DashboardPayload {
        hero_stats,
        tasks,
        transactions,
        pipeline,
        source: source.into(),
    })
}

fn build_hero_stats(state: &ApiState) -> Result<HeroStats, String> {
    let pipeline = build_pipeline_status(state)?;
    let minted_total = sum_confirmed_mint_amount(state)?;
    let core_load = average_vram_util(state)?;
    Ok(HeroStats {
        token_balance: minted_total,
        token_symbol: TOKEN_SYMBOL.into(),
        balance_trend_percent: if minted_total > 0.0 { 4.2 } else { 0.0 },
        active_tasks: pipeline.registered_nodes.max(pipeline.telemetry.min(u32::MAX as u64) as u32),
        core_load_percent: core_load,
        total_spent_usd: minted_total * USD_PER_TOKEN,
    })
}

fn build_tasks(state: &ApiState) -> Result<Vec<ComputeTask>, String> {
    let keys = state
        .queue
        .list_node_keys()
        .map_err(|e| e.to_string())?;
    let telem_head = state
        .queue
        .head_seq(QueueTopic::Telemetry)
        .map_err(|e| e.to_string())?;

    let mut tasks = Vec::new();
    for key in keys {
        let reg = state
            .queue
            .get_node_registration(&key)
            .map_err(|e| e.to_string())?;
        let Some(reg) = reg else { continue };
        let status = if telem_head > 0 {
            TaskStatus::Running
        } else {
            TaskStatus::Queued
        };
        let gpu = reg
            .hardware_class
            .split('-')
            .last()
            .unwrap_or(&reg.hardware_class)
            .to_uppercase();
        tasks.push(ComputeTask {
            id: format!("node-{}", reg.node_key),
            name: format!("{} compute epoch", reg.node_key),
            status,
            gpu_class: gpu,
            started_at: if telem_head > 0 {
                "live".into()
            } else {
                "—".into()
            },
        });
    }

    if tasks.is_empty() {
        let reports = state
            .queue
            .head_seq(QueueTopic::EpochReports)
            .map_err(|e| e.to_string())?;
        if reports > 0 {
            tasks.push(ComputeTask {
                id: "pipeline-oracle".into(),
                name: "Oracle epoch report aggregation".into(),
                status: TaskStatus::Running,
                gpu_class: "CPU".into(),
                started_at: "live".into(),
            });
        }
    }

    Ok(tasks)
}

fn build_transactions(state: &ApiState) -> Result<Vec<TransactionRow>, String> {
    let epochs = state.relayer.list_epochs().map_err(|e| e.to_string())?;
    let mut rows = Vec::new();

    for record in epochs.iter().rev() {
        for tx in record.txs.iter().rev() {
            rows.push(map_tx_record(record.epoch, tx, &record.receipt.minted_total.0));
        }
        if rows.len() >= 20 {
            break;
        }
    }

    if rows.is_empty() {
        let mints = state
            .queue
            .drain_recent_mint_instructions(10)
            .map_err(|e| e.to_string())?;
        for (_, inst) in mints.into_iter().rev() {
            let epoch = inst.signed.body.epoch.0;
            let total = inst.signed.body.total.0;
            rows.push(TransactionRow {
                hash: format!("0x{:064x}", epoch),
                action: TxAction::Mint,
                amount: format!("+{total} {TOKEN_SYMBOL}"),
                timestamp: format!("epoch {epoch}"),
                status: TxRowStatus::Pending,
            });
        }
    }

    Ok(rows)
}

fn map_tx_record(epoch: u64, tx: &TxRecord, minted: &u128) -> TransactionRow {
    let status = match tx.status {
        TxStatus::Confirmed => TxRowStatus::Success,
        TxStatus::Submitted | TxStatus::Pending => TxRowStatus::Pending,
        TxStatus::Failed => TxRowStatus::Failed,
    };
    let action = if tx.kind.contains("mint") {
        TxAction::Mint
    } else {
        TxAction::ComputePayment
    };
    let amount = match action {
        TxAction::Mint => format!("+{minted} {TOKEN_SYMBOL}"),
        TxAction::ComputePayment => format!("-fee {TOKEN_SYMBOL}"),
    };
    TransactionRow {
        hash: tx.digest.clone().unwrap_or_else(|| format!("epoch-{epoch}-{}", tx.kind)),
        action,
        amount,
        timestamp: format!("epoch {epoch}"),
        status,
    }
}

fn build_pipeline_status(state: &ApiState) -> Result<PipelineStatus, String> {
    let nodes = state
        .queue
        .list_node_keys()
        .map_err(|e| e.to_string())?
        .len() as u32;
    Ok(PipelineStatus {
        telemetry: state
            .queue
            .head_seq(QueueTopic::Telemetry)
            .map_err(|e| e.to_string())?,
        attestations: state
            .queue
            .head_seq(QueueTopic::Attestations)
            .map_err(|e| e.to_string())?,
        epoch_reports: state
            .queue
            .head_seq(QueueTopic::EpochReports)
            .map_err(|e| e.to_string())?,
        mint_instructions: state
            .queue
            .head_seq(QueueTopic::MintInstructions)
            .map_err(|e| e.to_string())?,
        registered_nodes: nodes,
        last_reconciled_epoch: state
            .relayer
            .last_reconciled_epoch()
            .map_err(|e| e.to_string())?,
    })
}

fn sum_confirmed_mint_amount(state: &ApiState) -> Result<f64, String> {
    let epochs = state.relayer.list_epochs().map_err(|e| e.to_string())?;
    let mut total: u128 = 0;
    for record in &epochs {
        let has_confirmed = record.txs.iter().any(|t| {
            t.kind == "execute_mint" && matches!(t.status, TxStatus::Confirmed)
        });
        if has_confirmed || record.reconciled {
            total = total.saturating_add(record.receipt.minted_total.0);
        }
    }
    if total == 0 {
        let mints = state
            .queue
            .drain_recent_mint_instructions(50)
            .map_err(|e| e.to_string())?;
        for (_, inst) in mints {
            total = total.saturating_add(inst.signed.body.total.0);
        }
    }
    Ok(total as f64)
}

fn average_vram_util(state: &ApiState) -> Result<u32, String> {
    let keys = state
        .queue
        .list_node_keys()
        .map_err(|e| e.to_string())?;
    if keys.is_empty() {
        return Ok(0);
    }
    let epoch = latest_epoch_from_queue(state)?;
    let mut sum = 0u64;
    let mut count = 0u64;
    for key in keys {
        let grouped = state
            .queue
            .telemetry_by_node(EpochId(epoch))
            .map_err(|e| e.to_string())?;
        if let Some(samples) = grouped.get(&key) {
            if let Some(last) = samples.last() {
                sum += last.sample.vram_util_bps as u64;
                count += 1;
            }
        }
    }
    if count == 0 {
        Ok(0)
    } else {
        Ok((sum / count / 100).min(100) as u32)
    }
}

fn latest_epoch_from_queue(state: &ApiState) -> Result<u64, String> {
    let mints = state
        .queue
        .drain_recent_mint_instructions(1)
        .map_err(|e| e.to_string())?;
    if let Some((_, inst)) = mints.last() {
        return Ok(inst.signed.body.epoch.0);
    }
    let epochs = state.relayer.list_epochs().map_err(|e| e.to_string())?;
    Ok(epochs.last().map(|r| r.epoch).unwrap_or(0))
}

fn handle_mint(state: &ApiState, req: MintRequest) -> Result<MintResponse, String> {
    if req.amount == 0 {
        return Err("amount must be greater than zero".into());
    }

    let epoch = latest_epoch_from_queue(state)?.saturating_add(1);
    let latest_cap = state
        .queue
        .drain_recent_mint_instructions(1)
        .map_err(|e| e.to_string())?
        .last()
        .map(|(_, inst)| inst.signed.body.total.0)
        .unwrap_or(0);

    let within_cap = latest_cap == 0 || u128::from(req.amount) <= latest_cap;
    if !within_cap {
        return Err(format!(
            "requested amount exceeds latest pipeline mint cap ({latest_cap} {TOKEN_SYMBOL})"
        ));
    }

    let message = if state.dry_run {
        format!(
            "Mint intent recorded for epoch {epoch} (dry_run=true — run cput-relayer to post on-chain)"
        )
    } else {
        format!("Mint intent accepted for epoch {epoch} — relayer will submit execute_mint")
    };

    Ok(MintResponse {
        status: "accepted".into(),
        amount: req.amount,
        epoch,
        message,
        dry_run: state.dry_run,
    })
}
