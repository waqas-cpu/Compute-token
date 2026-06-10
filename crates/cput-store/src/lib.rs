//! # cput-store — durable relayer & epoch state
//!
//! Backing store for production deployments: tracks posted transaction digests,
//! off-chain settlement receipts, and reconciliation outcomes per epoch.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use cput_core::ids::EpochId;
use cput_core::{CputError, CputResult};
use cput_gates::contracts::SettlementReceiptBody;
use serde::{Deserialize, Serialize};
use std::path::Path;

const TREE_EPOCHS: &[u8] = b"epochs";
const TREE_META: &[u8] = b"meta";

/// Lifecycle of an on-chain transaction posted by the relayer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxStatus {
    /// Intent recorded; not yet submitted.
    Pending,
    /// Submitted; awaiting finality.
    Submitted,
    /// Confirmed on-chain.
    Confirmed,
    /// Submission or execution failed.
    Failed,
}

/// A relayer transaction record keyed by epoch + kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRecord {
    /// Human-readable label (`submit_report`, `execute_mint`, etc.).
    pub kind: String,
    /// Sui transaction digest, if submitted.
    pub digest: Option<String>,
    /// Current status.
    pub status: TxStatus,
    /// Optional error detail.
    pub error: Option<String>,
}

/// Off-chain settlement snapshot stored for reconciliation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochRecord {
    /// Epoch identifier.
    pub epoch: u64,
    /// Off-chain settlement body (mirrors Gate 4→5 receipt).
    pub receipt: SettlementReceiptBody,
    /// Posted transaction records for this epoch.
    pub txs: Vec<TxRecord>,
    /// Whether on-chain reconciliation passed.
    pub reconciled: bool,
    /// Reconciliation detail (mismatch reason or OK).
    pub reconcile_note: Option<String>,
}

/// Persistent store for relayer operations.
pub struct RelayerStore {
    db: sled::Db,
}

impl RelayerStore {
    /// Open (or create) a store at `path`.
    pub fn open(path: impl AsRef<Path>) -> CputResult<Self> {
        let db = sled::open(path).map_err(|e| CputError::Store(e.to_string()))?;
        Ok(Self { db })
    }

    /// Last epoch fully reconciled on-chain.
    pub fn last_reconciled_epoch(&self) -> CputResult<Option<u64>> {
        let tree = self
            .db
            .open_tree(TREE_META)
            .map_err(|e| CputError::Store(e.to_string()))?;
        Ok(tree
            .get(b"last_reconciled_epoch")
            .map_err(|e| CputError::Store(e.to_string()))?
            .map(|v| serde_json::from_slice(&v))
            .transpose()
            .map_err(|e| CputError::Store(e.to_string()))?)
    }

    /// Update the last reconciled epoch cursor.
    pub fn set_last_reconciled_epoch(&self, epoch: u64) -> CputResult<()> {
        let tree = self
            .db
            .open_tree(TREE_META)
            .map_err(|e| CputError::Store(e.to_string()))?;
        tree.insert(
            b"last_reconciled_epoch",
            serde_json::to_vec(&epoch).map_err(|e| CputError::Store(e.to_string()))?,
        )
        .map_err(|e| CputError::Store(e.to_string()))?;
        tree.flush().map_err(|e| CputError::Store(e.to_string()))?;
        Ok(())
    }

    /// Persist an epoch record (receipt + tx status).
    pub fn put_epoch(&self, record: &EpochRecord) -> CputResult<()> {
        let tree = self
            .db
            .open_tree(TREE_EPOCHS)
            .map_err(|e| CputError::Store(e.to_string()))?;
        let key = record.epoch.to_le_bytes();
        tree.insert(
            key.as_slice(),
            serde_json::to_vec(record).map_err(|e| CputError::Store(e.to_string()))?,
        )
        .map_err(|e| CputError::Store(e.to_string()))?;
        tree.flush().map_err(|e| CputError::Store(e.to_string()))?;
        Ok(())
    }

    /// Load an epoch record.
    pub fn get_epoch(&self, epoch: EpochId) -> CputResult<Option<EpochRecord>> {
        let tree = self
            .db
            .open_tree(TREE_EPOCHS)
            .map_err(|e| CputError::Store(e.to_string()))?;
        Ok(tree
            .get(epoch.0.to_le_bytes())
            .map_err(|e| CputError::Store(e.to_string()))?
            .map(|v| serde_json::from_slice(&v))
            .transpose()
            .map_err(|e| CputError::Store(e.to_string()))?)
    }

    /// Record a transaction intent/digest for an epoch.
    pub fn append_tx(&self, epoch: EpochId, tx: TxRecord) -> CputResult<()> {
        let mut record = self
            .get_epoch(epoch)?
            .ok_or_else(|| CputError::Store(format!("no epoch record for {}", epoch.0)))?;
        record.txs.push(tx);
        self.put_epoch(&record)
    }

    /// List all persisted epoch records (sorted by epoch).
    pub fn list_epochs(&self) -> CputResult<Vec<EpochRecord>> {
        let tree = self
            .db
            .open_tree(TREE_EPOCHS)
            .map_err(|e| CputError::Store(e.to_string()))?;
        let mut out = Vec::new();
        for item in tree.iter() {
            let (_, v) = item.map_err(|e| CputError::Store(e.to_string()))?;
            let record: EpochRecord =
                serde_json::from_slice(&v).map_err(|e| CputError::Codec(e.to_string()))?;
            out.push(record);
        }
        out.sort_by_key(|r| r.epoch);
        Ok(out)
    }

    /// True if we already have a confirmed mint for this epoch.
    pub fn mint_posted(&self, epoch: EpochId) -> CputResult<bool> {
        Ok(self
            .get_epoch(epoch)?
            .map(|r| {
                r.txs.iter().any(|t| {
                    t.kind == "execute_mint" && matches!(t.status, TxStatus::Confirmed | TxStatus::Submitted)
                })
            })
            .unwrap_or(false))
    }
}
