//! SLH-DSA (FIPS 205, security level 5 — `SLH-DSA-SHA2-256s`).
//!
//! Role in CPUT: long-lived root-of-trust keys, genesis signing, audit-log
//! Merkle root signatures, and DAO governance actions. The `s` (small
//! signature) variant is chosen for compact, infrequently produced signatures
//! over long-lived artifacts.

use cput_core::{CputError, CputResult};
use fips205::slh_dsa_sha2_256s;
use fips205::traits::{SerDes, Signer, Verifier};

/// An SLH-DSA-SHA2-256s keypair.
pub struct SlhDsaKeypair {
    pk: slh_dsa_sha2_256s::PublicKey,
    sk: slh_dsa_sha2_256s::PrivateKey,
}

impl SlhDsaKeypair {
    /// Generate a fresh keypair using the system RNG.
    pub fn generate() -> CputResult<Self> {
        let (pk, sk) = slh_dsa_sha2_256s::try_keygen()
            .map_err(|e| CputError::KeyError(format!("slh-dsa keygen: {e}")))?;
        Ok(Self { pk, sk })
    }

    /// Sign `message` (hedged / randomised) returning raw signature bytes.
    pub fn sign(&self, message: &[u8]) -> CputResult<Vec<u8>> {
        let sig = self
            .sk
            .try_sign(message, &[], true)
            .map_err(|e| CputError::SignatureInvalid(format!("slh-dsa sign: {e}")))?;
        Ok(sig.to_vec())
    }

    /// The serialized public key.
    #[must_use]
    pub fn public_bytes(&self) -> Vec<u8> {
        self.pk.clone().into_bytes().to_vec()
    }
}

/// Verify an SLH-DSA signature given raw public-key and signature bytes.
pub fn verify(public_bytes: &[u8], message: &[u8], signature: &[u8]) -> CputResult<()> {
    let pk_arr: [u8; slh_dsa_sha2_256s::PK_LEN] = public_bytes
        .try_into()
        .map_err(|_| CputError::KeyError("slh-dsa public key wrong length".into()))?;
    let pk = slh_dsa_sha2_256s::PublicKey::try_from_bytes(&pk_arr)
        .map_err(|e| CputError::KeyError(format!("slh-dsa pk decode: {e}")))?;
    let sig_arr: [u8; slh_dsa_sha2_256s::SIG_LEN] = signature
        .try_into()
        .map_err(|_| CputError::SignatureInvalid("slh-dsa signature wrong length".into()))?;
    if pk.verify(message, &sig_arr, &[]) {
        Ok(())
    } else {
        Err(CputError::SignatureInvalid(
            "slh-dsa verification returned false".into(),
        ))
    }
}
