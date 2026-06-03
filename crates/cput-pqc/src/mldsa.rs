//! ML-DSA (FIPS 204, security level 5 — `ML-DSA-87`).
//!
//! Role in CPUT: node attestation signatures, agent mint-instruction signing,
//! oracle epoch-report signatures, and all smart-contract call authentication.

use cput_core::{CputError, CputResult};
use fips204::ml_dsa_87;
use fips204::traits::{SerDes, Signer, Verifier};

/// An ML-DSA-87 keypair.
pub struct MlDsaKeypair {
    pk: ml_dsa_87::PublicKey,
    sk: ml_dsa_87::PrivateKey,
}

impl MlDsaKeypair {
    /// Generate a fresh keypair using the system RNG.
    pub fn generate() -> CputResult<Self> {
        let (pk, sk) = ml_dsa_87::try_keygen()
            .map_err(|e| CputError::KeyError(format!("ml-dsa keygen: {e}")))?;
        Ok(Self { pk, sk })
    }

    /// Sign `message` with an empty context string, returning raw signature bytes.
    pub fn sign(&self, message: &[u8]) -> CputResult<Vec<u8>> {
        let sig = self
            .sk
            .try_sign(message, &[])
            .map_err(|e| CputError::SignatureInvalid(format!("ml-dsa sign: {e}")))?;
        Ok(sig.to_vec())
    }

    /// The serialized public key (the node/agent identity material).
    #[must_use]
    pub fn public_bytes(&self) -> Vec<u8> {
        self.pk.clone().into_bytes().to_vec()
    }
}

/// Verify an ML-DSA-87 signature given raw public-key and signature bytes.
pub fn verify(public_bytes: &[u8], message: &[u8], signature: &[u8]) -> CputResult<()> {
    let pk_arr: [u8; ml_dsa_87::PK_LEN] = public_bytes
        .try_into()
        .map_err(|_| CputError::KeyError("ml-dsa public key wrong length".into()))?;
    let pk = ml_dsa_87::PublicKey::try_from_bytes(pk_arr)
        .map_err(|e| CputError::KeyError(format!("ml-dsa pk decode: {e}")))?;
    let sig_arr: [u8; ml_dsa_87::SIG_LEN] = signature
        .try_into()
        .map_err(|_| CputError::SignatureInvalid("ml-dsa signature wrong length".into()))?;
    if pk.verify(message, &sig_arr, &[]) {
        Ok(())
    } else {
        Err(CputError::SignatureInvalid(
            "ml-dsa verification returned false".into(),
        ))
    }
}
