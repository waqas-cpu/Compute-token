//! Groth16 proof serialization and batch aggregation encoding.

use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, Proof as Groth16Proof, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use cput_core::{CputError, CputResult};
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 8] = b"CPUTZK01";

/// Serialize a Groth16 proof to canonical bytes.
pub fn serialize_groth16(proof: &Groth16Proof<Bn254>) -> CputResult<Vec<u8>> {
    let mut bytes = Vec::new();
    proof
        .serialize_compressed(&mut bytes)
        .map_err(|e| CputError::ProofInvalid(format!("groth16 encode: {e}")))?;
    Ok(bytes)
}

/// Deserialize a Groth16 proof.
pub fn deserialize_groth16(bytes: &[u8]) -> CputResult<Groth16Proof<Bn254>> {
    Groth16Proof::deserialize_compressed(bytes)
        .map_err(|e| CputError::ProofInvalid(format!("groth16 decode: {e}")))
}

/// Verify a single Groth16 proof against public inputs.
pub fn verify_groth16(
    vk: &VerifyingKey<Bn254>,
    public_inputs: &[Fr],
    proof: &Groth16Proof<Bn254>,
) -> CputResult<()> {
    let ok = Groth16::<Bn254>::verify(vk, public_inputs, proof)
        .map_err(|e| CputError::ProofInvalid(format!("groth16 verify: {e}")))?;
    if ok {
        Ok(())
    } else {
        Err(CputError::ProofInvalid(
            "groth16 verification returned false".into(),
        ))
    }
}

/// Commitment over canonical statement bytes (on-chain `zk_proof_hash`).
pub fn public_commitment(public_inputs: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"cput.zk.groth16.leaf.v1");
    h.update((public_inputs.len() as u64).to_le_bytes());
    h.update(public_inputs);
    h.finalize().into()
}

/// Pack multiple leaf proofs into one aggregate blob.
pub fn pack_batch(leaves: &[Vec<u8>]) -> CputResult<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(leaves.len() as u32).to_le_bytes());
    for leaf in leaves {
        let len = u32::try_from(leaf.len())
            .map_err(|_| CputError::ProofInvalid("leaf proof too large".into()))?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(leaf);
    }
    Ok(out)
}

/// Unpack a batch aggregate blob.
pub fn unpack_batch(bytes: &[u8]) -> CputResult<Vec<Vec<u8>>> {
    if bytes.len() < 12 || &bytes[..8] != MAGIC {
        return Err(CputError::ProofInvalid("invalid aggregate magic".into()));
    }
    let count = u32::from_le_bytes(bytes[8..12].try_into().expect("count bytes")) as usize;
    let mut offset = 12;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if offset + 4 > bytes.len() {
            return Err(CputError::ProofInvalid("truncated aggregate batch".into()));
        }
        let len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("len")) as usize;
        offset += 4;
        if offset + len > bytes.len() {
            return Err(CputError::ProofInvalid("truncated leaf proof".into()));
        }
        out.push(bytes[offset..offset + len].to_vec());
        offset += len;
    }
    Ok(out)
}

/// Aggregate commitment over leaf public-input commitments.
pub fn aggregate_commitment(leaf_commitments: &[[u8; 32]]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"cput.zk.groth16.aggregate.v1");
    for c in leaf_commitments {
        h.update(c);
    }
    h.finalize().into()
}
