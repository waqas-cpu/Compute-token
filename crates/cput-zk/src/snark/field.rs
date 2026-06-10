//! Field-element helpers for BN254 Groth16 circuits.

use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};

/// Map a `u128` protocol quantity into the BN254 scalar field.
#[must_use]
pub fn u128_to_fr(value: u128) -> Fr {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(&value.to_le_bytes());
    Fr::from_le_bytes_mod_order(&bytes)
}

/// Map a 32-byte digest into the scalar field (big-endian interpretation).
#[must_use]
pub fn bytes32_to_fr(bytes: [u8; 32]) -> Fr {
    Fr::from_le_bytes_mod_order(&bytes)
}

/// Expected workload trace binding used by the compute circuit.
#[must_use]
pub fn workload_binding_fr(workload_hash: [u8; 32]) -> Fr {
    use sha2::{Digest, Sha256};
    let digest: [u8; 32] = {
        let mut h = Sha256::new();
        h.update(b"cput.zk.trace.root.v1");
        h.update(workload_hash);
        h.finalize().into()
    };
    bytes32_to_fr(digest)
}

/// Serialize public Fr values for commitment hashing.
pub fn fr_slice_to_bytes(values: &[Fr]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 32);
    for v in values {
        out.extend_from_slice(&v.into_bigint().to_bytes_le());
    }
    out
}
