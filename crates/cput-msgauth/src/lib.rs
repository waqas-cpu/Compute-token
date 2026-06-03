//! # cput-msgauth — Cross-layer message authentication & anti-replay (HORIZONTAL)
//!
//! Implements the Layer 3 rule: *"Every message crossing a layer boundary
//! carries a MAC derived from the sender's ML-DSA key and the message content.
//! Replay attacks are prevented by including epoch number and a monotonic
//! sequence counter in the authenticated data."*
//!
//! The MAC is an ML-DSA-87 signature over a domain-separated, length-prefixed
//! encoding of `(epoch, sequence, payload)`. The [`ReplayGuard`] enforces
//! strictly-increasing sequence numbers per sender.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use cput_core::ids::EpochId;
use cput_core::{CputError, CputResult};
use cput_pqc::mldsa::{self, MlDsaKeypair};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DOMAIN: &[u8] = b"cput.msgauth.v1";

/// A message authenticated for transit across a layer boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedMessage {
    /// Sender's ML-DSA-87 public key (identity).
    pub sender_public: Vec<u8>,
    /// Epoch the message belongs to (part of authenticated data).
    pub epoch: EpochId,
    /// Monotonic per-sender sequence counter (anti-replay).
    pub sequence: u64,
    /// Opaque payload.
    pub payload: Vec<u8>,
    /// ML-DSA signature (the MAC) over the authenticated bytes.
    pub mac: Vec<u8>,
}

fn authenticated_bytes(epoch: EpochId, sequence: u64, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(DOMAIN.len() + 24 + payload.len());
    buf.extend_from_slice(DOMAIN);
    buf.extend_from_slice(&epoch.0.to_le_bytes());
    buf.extend_from_slice(&sequence.to_le_bytes());
    buf.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    buf.extend_from_slice(payload);
    buf
}

impl AuthenticatedMessage {
    /// Authenticate a payload for the given epoch and sequence number.
    pub fn seal(
        keypair: &MlDsaKeypair,
        epoch: EpochId,
        sequence: u64,
        payload: Vec<u8>,
    ) -> CputResult<Self> {
        let mac = keypair.sign(&authenticated_bytes(epoch, sequence, &payload))?;
        Ok(Self {
            sender_public: keypair.public_bytes(),
            epoch,
            sequence,
            payload,
            mac,
        })
    }

    /// Verify the MAC (does not check replay; use a [`ReplayGuard`] for that).
    pub fn verify_mac(&self) -> CputResult<()> {
        let bytes = authenticated_bytes(self.epoch, self.sequence, &self.payload);
        mldsa::verify(&self.sender_public, &bytes, &self.mac)
    }
}

/// Tracks the highest accepted sequence number per sender to reject replays
/// and out-of-order delivery.
#[derive(Debug, Default)]
pub struct ReplayGuard {
    last_seq: HashMap<Vec<u8>, u64>,
}

impl ReplayGuard {
    /// Create an empty guard.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Verify the MAC and enforce a strictly-increasing sequence for the
    /// sender. On success the sender's high-water mark is advanced.
    pub fn admit(&mut self, msg: &AuthenticatedMessage) -> CputResult<()> {
        msg.verify_mac()?;
        let entry = self.last_seq.get(&msg.sender_public).copied();
        if let Some(prev) = entry {
            if msg.sequence <= prev {
                return Err(CputError::Replay(format!(
                    "sequence {} not greater than last accepted {}",
                    msg.sequence, prev
                )));
            }
        }
        self.last_seq
            .insert(msg.sender_public.clone(), msg.sequence);
        Ok(())
    }
}
