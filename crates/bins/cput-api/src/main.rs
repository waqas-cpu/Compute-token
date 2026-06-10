//! `cput-api` — REST bridge between the Next.js dashboard and off-chain stores.

use clap::Parser;
use cput_api::{router, ApiConfig, ApiState};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "cput-api", about = "HTTP API for the Compute Token dashboard")]
struct Args {
    /// Pipeline queue directory.
    #[arg(long, default_value = "data/pipeline")]
    queue: PathBuf,
    /// Relayer sled store directory.
    #[arg(long, default_value = "data/relayer")]
    relayer: PathBuf,
    /// Optional Sui deployment TOML (`config/cput.toml`).
    #[arg(long)]
    config: Option<PathBuf>,
    /// Listen address.
    #[arg(long, default_value = "0.0.0.0:8787")]
    listen: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cput_api=info,tower_http=info".into()),
        )
        .init();

    let args = Args::parse();
    let cfg = ApiConfig {
        queue_path: args.queue,
        relayer_path: args.relayer,
        sui_config: args.config,
        listen_addr: args.listen,
    };

    let state = ApiState::open(&cfg)?;
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(&cfg.listen_addr).await?;
    tracing::info!("cput-api listening on http://{}", cfg.listen_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
