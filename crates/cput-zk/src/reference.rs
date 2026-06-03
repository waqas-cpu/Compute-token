//! Deterministic reference backend.
//!
//! **NOT zero-knowledge and NOT cryptographically binding.** It commits to the
//! public inputs with SHA-256 so the end-to-end pipeline (prove → aggregate →
//! verify) is exercisable offline. Replace with Barretenberg/Noir in
//! production (see `PRODUCTION.md`).

use crate::proof::{Proof, ProofBackend, ProofSystemId};
use cput_core::{CputError, CputResult};
use sha2::{Digest, Sha256};

/// Domain separation tags so commitments are not interchangeable across uses.
const TAG_LEAF: &[u8] = b"cput.zk.reference.leaf.v1";
const TAG_AGG: &[u8] = b"cput.zk.reference.aggregate.v1";

fn commit(tag: &[u8], data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(tag);
    h.update((data.len() as u64).to_le_bytes());
    h.update(data);
    h.finalize().into()
}

/// The reference proving backend. Carries no proving key — it is pure hashing.
#[derive(Debug, Clone, Default)]
pub struct ReferenceBackend;

impl ProofBackend for ReferenceBackend {
    fn prove(&self, public_inputs: &[u8]) -> CputResult<Proof> {
        let c = commit(TAG_LEAF, public_inputs);
        Ok(Proof {
            system: ProofSystemId::Reference,
            public_inputs_commitment: c,
            bytes: c.to_vec(),
            folded: 1,
        })
    }

    fn verify(&self, proof: &Proof, public_inputs: &[u8]) -> CputResult<()> {
        if proof.system != ProofSystemId::Reference {
            return Err(CputError::ProofInvalid(
                "backend/proof system mismatch".into(),
            ));
        }
        let expected = commit(TAG_LEAF, public_inputs);
        if proof.public_inputs_commitment == expected && proof.bytes == expected {
            Ok(())
        } else {
            Err(CputError::ProofInvalid(
                "reference commitment mismatch".into(),
            ))
        }
    }

    fn aggregate(&self, proofs: &[Proof]) -> CputResult<Proof> {
        if proofs.is_empty() {
            return Err(CputError::ProofInvalid(
                "cannot aggregate zero proofs".into(),
            ));
        }
        let mut buf = Vec::with_capacity(proofs.len() * 32);
        let mut folded = 0u32;
        for p in proofs {
            // A well-formed reference leaf stores its commitment as `bytes`.
            if p.system != ProofSystemId::Reference || p.bytes != p.public_inputs_commitment {
                return Err(CputError::ProofInvalid(
                    "malformed leaf proof in batch".into(),
                ));
            }
            buf.extend_from_slice(&p.public_inputs_commitment);
            folded += p.folded;
        }
        let c = commit(TAG_AGG, &buf);
        Ok(Proof {
            system: ProofSystemId::Reference,
            public_inputs_commitment: c,
            bytes: buf,
            folded,
        })
    }

    fn verify_aggregate(&self, aggregate: &Proof, batch_inputs: &[Vec<u8>]) -> CputResult<()> {
        if aggregate.system != ProofSystemId::Reference {
            return Err(CputError::ProofInvalid(
                "backend/proof system mismatch".into(),
            ));
        }
        let mut buf = Vec::with_capacity(batch_inputs.len() * 32);
        for inputs in batch_inputs {
            buf.extend_from_slice(&commit(TAG_LEAF, inputs));
        }
        let expected = commit(TAG_AGG, &buf);
        if aggregate.public_inputs_commitment == expected && aggregate.bytes == buf {
            Ok(())
        } else {
            Err(CputError::ProofInvalid(
                "aggregate commitment mismatch".into(),
            ))
        }
    }
}
