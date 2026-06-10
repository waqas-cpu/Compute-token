//! Sui JSON-RPC client for read-only queries (events, objects).

use crate::config::SuiDeployment;
use crate::types::{OnChainEpochMinted, OnChainFeeRouted};
use cput_core::{CputError, CputResult};
use serde::Deserialize;
use serde_json::json;

/// Minimal JSON-RPC client for reconciliation queries.
pub struct RpcClient {
    rpc_url: String,
    package_id: String,
}

#[derive(Debug, Deserialize)]
struct RpcEnvelope<T> {
    result: Option<T>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct QueryEventsResult {
    data: Vec<SuiEvent>,
}

#[derive(Debug, Deserialize)]
struct SuiEvent {
    #[serde(default)]
    parsed_json: Option<serde_json::Value>,
}

impl RpcClient {
    /// Construct a client from deployment config.
    pub fn new(cfg: &SuiDeployment) -> CputResult<Self> {
        Ok(Self {
            rpc_url: cfg.rpc_url.clone(),
            package_id: cfg.package_id.clone(),
        })
    }

    fn call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> CputResult<T> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let resp = ureq::post(&self.rpc_url)
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(|e| CputError::Chain(e.to_string()))?;
        let resp: RpcEnvelope<T> = resp
            .into_json()
            .map_err(|e| CputError::Chain(e.to_string()))?;
        if let Some(err) = resp.error {
            return Err(CputError::Chain(format!("rpc error: {err}")));
        }
        resp.result
            .ok_or_else(|| CputError::Chain("empty rpc result".into()))
    }

    /// Fetch `EpochMinted` events for a specific epoch from the `minting` module.
    pub fn epoch_minted(&self, epoch: u64) -> CputResult<Option<OnChainEpochMinted>> {
        let filter = json!({
            "MoveEventType": format!("{}::minting::EpochMinted", self.package_id)
        });
        let result: QueryEventsResult = self.call(
            "suix_queryEvents",
            json!([filter, null, 50, false]),
        )?;
        for ev in result.data {
            if let Some(parsed) = ev.parsed_json {
                if parsed.get("epoch").and_then(|v| v.as_u64()) == Some(epoch) {
                    return Ok(Some(parse_epoch_minted(&parsed)?));
                }
            }
        }
        Ok(None)
    }

    /// Fetch the latest `FeeRouted` event for a payer (optional reconciliation).
    pub fn latest_fee_routed(&self, payer: &str) -> CputResult<Option<OnChainFeeRouted>> {
        let filter = json!({
            "MoveEventType": format!("{}::burn::FeeRouted", self.package_id)
        });
        let result: QueryEventsResult = self.call(
            "suix_queryEvents",
            json!([filter, null, 20, false]),
        )?;
        for ev in result.data {
            if let Some(parsed) = ev.parsed_json {
                if parsed.get("payer").and_then(|v| v.as_str()) == Some(payer) {
                    return Ok(Some(parse_fee_routed(&parsed)?));
                }
            }
        }
        Ok(None)
    }
}

fn parse_epoch_minted(v: &serde_json::Value) -> CputResult<OnChainEpochMinted> {
    let get_u64 = |k: &str| -> CputResult<u64> {
        v.get(k)
            .and_then(|x| x.as_str().and_then(|s| s.parse().ok()).or_else(|| x.as_u64()))
            .ok_or_else(|| CputError::Chain(format!("missing field {k} in EpochMinted")))
    };
    Ok(OnChainEpochMinted {
        epoch: get_u64("epoch")?,
        total: get_u64("total")?,
        providers: get_u64("providers")?,
        oracle: get_u64("oracle")?,
        treasury: get_u64("treasury")?,
        burn_reserve: get_u64("burn_reserve")?,
        verified_gflops: get_u64("verified_gflops")?,
    })
}

fn parse_fee_routed(v: &serde_json::Value) -> CputResult<OnChainFeeRouted> {
    let get_u64 = |k: &str| -> CputResult<u64> {
        v.get(k)
            .and_then(|x| x.as_str().and_then(|s| s.parse().ok()).or_else(|| x.as_u64()))
            .ok_or_else(|| CputError::Chain(format!("missing field {k} in FeeRouted")))
    };
    let payer = v
        .get("payer")
        .and_then(|x| x.as_str())
        .ok_or_else(|| CputError::Chain("missing payer in FeeRouted".into()))?
        .to_string();
    Ok(OnChainFeeRouted {
        payer,
        fee: get_u64("fee")?,
        burned: get_u64("burned")?,
        providers: get_u64("providers")?,
        treasury: get_u64("treasury")?,
    })
}
