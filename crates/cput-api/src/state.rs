//! Shared API state (queue, relayer store, optional Sui deployment).

use cput_queue::PipelineQueue;
use cput_store::RelayerStore;
use cput_sui::config::SuiDeployment;
use cput_sui::rpc::RpcClient;
use std::path::PathBuf;
use std::sync::Arc;

/// Paths and optional chain config for the API server.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Pipeline queue sled directory (`data/pipeline`).
    pub queue_path: PathBuf,
    /// Relayer store sled directory (`data/relayer`).
    pub relayer_path: PathBuf,
    /// Optional `config/cput.toml` for on-chain reads.
    pub sui_config: Option<PathBuf>,
    /// Bind address (e.g. `0.0.0.0:8787`).
    pub listen_addr: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            queue_path: PathBuf::from("data/pipeline"),
            relayer_path: PathBuf::from("data/relayer"),
            sui_config: None,
            listen_addr: "0.0.0.0:8787".into(),
        }
    }
}

/// Thread-safe handles opened at startup.
pub struct ApiState {
    pub(crate) queue: PipelineQueue,
    pub(crate) relayer: RelayerStore,
    pub(crate) rpc: Option<RpcClient>,
    pub(crate) dry_run: bool,
}

impl ApiState {
    /// Open stores and optionally load Sui deployment config.
    pub fn open(cfg: &ApiConfig) -> Result<Arc<Self>, String> {
        let queue = PipelineQueue::open(&cfg.queue_path)
            .map_err(|e| format!("open queue {}: {e}", cfg.queue_path.display()))?;
        let relayer = RelayerStore::open(&cfg.relayer_path)
            .map_err(|e| format!("open relayer {}: {e}", cfg.relayer_path.display()))?;

        let (rpc, dry_run) = if let Some(path) = &cfg.sui_config {
            match SuiDeployment::from_file(path) {
                Ok(dep) => {
                    let dry = dep.dry_run;
                    let rpc = RpcClient::new(&dep).ok();
                    (rpc, dry)
                }
                Err(e) => {
                    tracing::warn!("sui config {path:?}: {e}");
                    (None, true)
                }
            }
        } else {
            (None, true)
        };

        Ok(Arc::new(Self {
            queue,
            relayer,
            rpc,
            dry_run,
        }))
    }
}
