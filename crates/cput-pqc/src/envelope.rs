//! Algorithm-tagged, signed envelopes — the unit that crosses every gate.
//!
//! A [`SealedEnvelope`] binds a payload to (a) the algorithm used, (b) the
//! signer's public key, and (c) a PQC signature. Integration gates verify
//! envelopes before admitting a payload into the next layer, and check the
//! algorithm against the crypto-agility [`AlgorithmRegistry`].

use crate::registry::{AlgorithmId, AlgorithmRegistry};
use crate::{mldsa, slhdsa};
use cput_core::{CputError, CputResult};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// A signed, algorithm-tagged container for an opaque payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedEnvelope {
    /// Signature algorithm used to seal the payload.
    pub algorithm: AlgorithmId,
    /// Serialized public key of the signer (identity material).
    pub signer_public: Vec<u8>,
    /// Canonical serialized payload bytes.
    pub payload: Vec<u8>,
    /// PQC signature over `payload`.
    pub signature: Vec<u8>,
}

impl SealedEnvelope {
    /// Seal a payload with an ML-DSA-87 keypair (the common case).
    pub fn seal_mldsa(keypair: &mldsa::MlDsaKeypair, payload: Vec<u8>) -> CputResult<Self> {
        let signature = keypair.sign(&payload)?;
        Ok(Self {
            algorithm: AlgorithmId::MlDsa87,
            signer_public: keypair.public_bytes(),
            payload,
            signature,
        })
    }

    /// Seal a long-lived artifact with an SLH-DSA keypair.
    pub fn seal_slhdsa(keypair: &slhdsa::SlhDsaKeypair, payload: Vec<u8>) -> CputResult<Self> {
        let signature = keypair.sign(&payload)?;
        Ok(Self {
            algorithm: AlgorithmId::SlhDsaSha2_256s,
            signer_public: keypair.public_bytes(),
            payload,
            signature,
        })
    }

    /// Verify the envelope signature, enforcing the crypto-agility registry.
    pub fn verify(&self, registry: &AlgorithmRegistry) -> CputResult<()> {
        registry.require_approved(self.algorithm)?;
        match self.algorithm {
            AlgorithmId::MlDsa87 => {
                mldsa::verify(&self.signer_public, &self.payload, &self.signature)
            }
            AlgorithmId::SlhDsaSha2_256s => {
                slhdsa::verify(&self.signer_public, &self.payload, &self.signature)
            }
            AlgorithmId::MlKem1024 => Err(CputError::RuleViolation {
                rule: "PQC.ENVELOPE.SIGN_ALGO",
                detail: "ML-KEM is a KEM, not a signature scheme".into(),
            }),
        }
    }
}

/// A typed body bound to a PQC signature over its canonical (JSON) encoding.
///
/// This is the building block for every cross-layer boundary contract: the
/// producing layer seals a `T`, and the receiving integration gate verifies
/// both that the signature is valid *and* that it covers exactly the body
/// presented.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signed<T> {
    /// The signed message body.
    pub body: T,
    /// The PQC signature envelope over `canonical(body)`.
    pub envelope: SealedEnvelope,
}

fn canonical<T: Serialize>(body: &T) -> CputResult<Vec<u8>> {
    serde_json::to_vec(body).map_err(|e| CputError::Codec(format!("canonical encode: {e}")))
}

impl<T: Serialize + DeserializeOwned> Signed<T> {
    /// Seal `body` with an ML-DSA-87 keypair.
    pub fn seal_mldsa(keypair: &mldsa::MlDsaKeypair, body: T) -> CputResult<Self> {
        let payload = canonical(&body)?;
        let envelope = SealedEnvelope::seal_mldsa(keypair, payload)?;
        Ok(Self { body, envelope })
    }

    /// Seal `body` with an SLH-DSA keypair (long-lived / governance artifacts).
    pub fn seal_slhdsa(keypair: &slhdsa::SlhDsaKeypair, body: T) -> CputResult<Self> {
        let payload = canonical(&body)?;
        let envelope = SealedEnvelope::seal_slhdsa(keypair, payload)?;
        Ok(Self { body, envelope })
    }

    /// Verify the signature and that it covers exactly this body.
    pub fn verify(&self, registry: &AlgorithmRegistry) -> CputResult<()> {
        let expected = canonical(&self.body)?;
        if self.envelope.payload != expected {
            return Err(CputError::SignatureInvalid(
                "envelope payload does not match signed body".into(),
            ));
        }
        self.envelope.verify(registry)
    }

    /// The signer's public key bytes (identity of the producing party).
    #[must_use]
    pub fn signer_public(&self) -> &[u8] {
        &self.envelope.signer_public
    }
}
