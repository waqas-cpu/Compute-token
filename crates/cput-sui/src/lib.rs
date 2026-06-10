//! # cput-sui — Sui network integration layer
//!
//! Bridges the off-chain Rust pipeline (L1–L5) to the `sui-move/` on-chain
//! package: builds transaction intents, submits via the Sui CLI, queries events
//! for reconciliation, and persists relayer state through [`cput_store`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod adapter;
pub mod config;
pub mod reconcile;
pub mod relay;
pub mod rpc;
pub mod sponsor;
pub mod types;

pub use adapter::{
    agent_quorum_from_instruction, execute_mint_from_instruction, expected_epoch_minted,
    expected_fee_routed, set_ceiling_from_policy, submit_report_from_epoch,
};
pub use config::{PoolAddresses, SuiDeployment};
pub use reconcile::{reconcile_epoch_mint, reconcile_fee_routed};
pub use relay::SuiRelayer;
pub use sponsor::SponsorConfig;
pub use rpc::RpcClient;
pub use types::{
    AgentQuorumIntent, ExecuteMintIntent, OnChainEpochMinted, OnChainFeeRouted,
    PayAccessFeeIntent, SetCeilingIntent, SubmitReportIntent,
};
