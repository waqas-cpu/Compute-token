//! ML-KEM (FIPS 203, security level 5 — `ML-KEM-1024`).
//!
//! Role in CPUT: key encapsulation for all encrypted node↔oracle and agent
//! key-exchange channels, and forward-secret session keys backing the
//! Harvest-Now-Decrypt-Later (HNDL) guard.

use cput_core::{CputError, CputResult};
use fips203::ml_kem_1024;
use fips203::traits::{Decaps, Encaps, KeyGen, SerDes};

/// An ML-KEM-1024 keypair (encapsulation + decapsulation keys).
pub struct MlKemKeypair {
    ek: ml_kem_1024::EncapsKey,
    dk: ml_kem_1024::DecapsKey,
}

/// The result of encapsulating to a peer: the ciphertext to transmit and the
/// 32-byte shared secret the sender derives locally.
pub struct Encapsulation {
    /// Ciphertext to transmit to the holder of the decapsulation key.
    pub ciphertext: Vec<u8>,
    /// Shared secret derived by the encapsulating party.
    pub shared_secret: [u8; 32],
}

impl MlKemKeypair {
    /// Generate a fresh keypair using the system RNG.
    pub fn generate() -> CputResult<Self> {
        let (ek, dk) = ml_kem_1024::KG::try_keygen()
            .map_err(|e| CputError::KeyError(format!("ml-kem keygen: {e}")))?;
        Ok(Self { ek, dk })
    }

    /// Serialized encapsulation (public) key, published so peers can encapsulate.
    #[must_use]
    pub fn encaps_public_bytes(&self) -> Vec<u8> {
        self.ek.clone().into_bytes().to_vec()
    }

    /// Decapsulate a received ciphertext into the shared secret.
    pub fn decapsulate(&self, ciphertext: &[u8]) -> CputResult<[u8; 32]> {
        let ct_arr: [u8; ml_kem_1024::CT_LEN] = ciphertext
            .try_into()
            .map_err(|_| CputError::KemFailure("ml-kem ciphertext wrong length".into()))?;
        let ct = ml_kem_1024::CipherText::try_from_bytes(ct_arr)
            .map_err(|e| CputError::KemFailure(format!("ml-kem ct decode: {e}")))?;
        let ssk = self
            .dk
            .try_decaps(&ct)
            .map_err(|e| CputError::KemFailure(format!("ml-kem decaps: {e}")))?;
        Ok(ssk.into_bytes())
    }
}

/// Encapsulate a fresh shared secret to a peer's published encapsulation key.
pub fn encapsulate_to(encaps_public_bytes: &[u8]) -> CputResult<Encapsulation> {
    let ek_arr: [u8; ml_kem_1024::EK_LEN] = encaps_public_bytes
        .try_into()
        .map_err(|_| CputError::KemFailure("ml-kem encaps key wrong length".into()))?;
    let ek = ml_kem_1024::EncapsKey::try_from_bytes(ek_arr)
        .map_err(|e| CputError::KemFailure(format!("ml-kem ek decode: {e}")))?;
    let (ssk, ct) = ek
        .try_encaps()
        .map_err(|e| CputError::KemFailure(format!("ml-kem encaps: {e}")))?;
    Ok(Encapsulation {
        ciphertext: ct.into_bytes().to_vec(),
        shared_secret: ssk.into_bytes(),
    })
}
