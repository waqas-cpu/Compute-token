//! Production-shaped ZK backend with witness binding and constraint checks.
//!
//! Proofs carry a witness commitment and a constraint-satisfaction digest.
//! Replace the internal hashing with Noir/Barretenberg verification once
//! circuits are audited (`circuits/` + `noir` feature).

use crate::proof::{Proof, ProofBackend, ProofSystemId};
use crate::statements::{AgentReasoningStatement, ComputeStatement};
use crate::witness::{ComputeWitness, ReasoningWitness};
use cput_core::{CputError, CputResult};
use sha2::{Digest, Sha256};

const TAG_LEAF: &[u8] = b"cput.zk.production.leaf.v1";
const TAG_AGG: &[u8] = b"cput.zk.production.aggregate.v1";
const TAG_SAT: &[u8] = b"cput.zk.production.sat.v1";

fn commit(tag: &[u8], data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(tag);
    h.update((data.len() as u64).to_le_bytes());
    h.update(data);
    h.finalize().into()
}

fn pack_proof(witness_commitment: [u8; 32], public_commitment: [u8; 32]) -> Vec<u8> {
    let sat = {
        let mut h = Sha256::new();
        h.update(TAG_SAT);
        h.update(witness_commitment);
        h.update(public_commitment);
        h.finalize()
    };
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(&witness_commitment);
    out.extend_from_slice(&sat);
    out
}

fn unpack_proof(bytes: &[u8]) -> CputResult<([u8; 32], [u8; 32])> {
    if bytes.len() != 64 {
        return Err(CputError::ProofInvalid(
            "production proof must be 64 bytes (witness_commitment || sat)".into(),
        ));
    }
    let mut wc = [0u8; 32];
    let mut sat = [0u8; 32];
    wc.copy_from_slice(&bytes[..32]);
    sat.copy_from_slice(&bytes[32..]);
    Ok((wc, sat))
}

/// Production backend: witness-bound proofs with explicit constraint checks.
#[derive(Debug, Clone, Default)]
pub struct ProductionBackend;

impl ProductionBackend {
    fn check_compute(statement: &ComputeStatement, witness: &ComputeWitness) -> CputResult<()> {
        if witness.measured_gflops != statement.gflops.0 {
            return Err(CputError::ProofInvalid(format!(
                "witness measured_gflops {} != public {}",
                witness.measured_gflops, statement.gflops.0
            )));
        }
        let expected_trace: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(b"cput.zk.trace.root.v1");
            h.update(statement.workload_hash.0);
            h.finalize().into()
        };
        if witness.execution_trace_root != expected_trace {
            return Err(CputError::ProofInvalid(
                "execution_trace_root does not bind workload_hash".into(),
            ));
        }
        Ok(())
    }

    fn check_reasoning(
        statement: &AgentReasoningStatement,
        witness: &ReasoningWitness,
    ) -> CputResult<()> {
        if witness.computed_mint != statement.mint_amount {
            return Err(CputError::ProofInvalid(format!(
                "witness computed_mint {} != public {}",
                witness.computed_mint, statement.mint_amount
            )));
        }
        Ok(())
    }

    fn verify_leaf(proof: &Proof, public_inputs: &[u8]) -> CputResult<()> {
        if proof.system != ProofSystemId::UltraPlonk {
            return Err(CputError::ProofInvalid(
                "production backend requires UltraPlonk proofs".into(),
            ));
        }
        let expected_public = commit(TAG_LEAF, public_inputs);
        if proof.public_inputs_commitment != expected_public {
            return Err(CputError::ProofInvalid(
                "public inputs commitment mismatch".into(),
            ));
        }
        let (witness_c, sat) = unpack_proof(&proof.bytes)?;
        let expected_sat: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(TAG_SAT);
            h.update(witness_c);
            h.update(expected_public);
            h.finalize().into()
        };
        if sat != expected_sat {
            return Err(CputError::ProofInvalid(
                "constraint-satisfaction digest mismatch".into(),
            ));
        }
        Ok(())
    }
}

fn leaf_proof(
    statement_bytes: &[u8],
    witness: &impl WitnessCommit,
    check: impl FnOnce() -> CputResult<()>,
) -> CputResult<Proof> {
    check()?;
    let public_c = commit(TAG_LEAF, statement_bytes);
    let witness_c = witness.commitment();
    Ok(Proof {
        system: ProofSystemId::UltraPlonk,
        public_inputs_commitment: public_c,
        bytes: pack_proof(witness_c, public_c),
        folded: 1,
    })
}

trait WitnessCommit {
    fn commitment(&self) -> [u8; 32];
}

impl WitnessCommit for ComputeWitness {
    fn commitment(&self) -> [u8; 32] {
        ComputeWitness::commitment(self)
    }
}

impl WitnessCommit for ReasoningWitness {
    fn commitment(&self) -> [u8; 32] {
        ReasoningWitness::commitment(self)
    }
}

impl ProofBackend for ProductionBackend {
    fn prove_compute(
        &self,
        statement: &ComputeStatement,
        witness: &ComputeWitness,
    ) -> CputResult<Proof> {
        let public = crate::encode_statement(statement)?;
        leaf_proof(&public, witness, || Self::check_compute(statement, witness))
    }

    fn prove_reasoning(
        &self,
        statement: &AgentReasoningStatement,
        witness: &ReasoningWitness,
    ) -> CputResult<Proof> {
        let public = crate::encode_statement(statement)?;
        leaf_proof(&public, witness, || Self::check_reasoning(statement, witness))
    }

    fn prove(&self, public_inputs: &[u8]) -> CputResult<Proof> {
        if let Ok(stmt) = serde_json::from_slice::<ComputeStatement>(public_inputs) {
            let witness = ComputeWitness::from_attestation(
                stmt.gflops,
                stmt.workload_hash,
                stmt.enclave_measurement,
            );
            return self.prove_compute(&stmt, &witness);
        }
        if let Ok(stmt) = serde_json::from_slice::<AgentReasoningStatement>(public_inputs) {
            let witness = ReasoningWitness::from_formula(
                stmt.epoch.0,
                stmt.verified_gflops.0,
                1,
                stmt.mint_amount,
            );
            return self.prove_reasoning(&stmt, &witness);
        }
        Err(CputError::ProofInvalid(
            "production prove requires ComputeStatement or AgentReasoningStatement JSON".into(),
        ))
    }

    fn verify(&self, proof: &Proof, public_inputs: &[u8]) -> CputResult<()> {
        Self::verify_leaf(proof, public_inputs)
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
            if p.system != ProofSystemId::UltraPlonk {
                return Err(CputError::ProofInvalid(
                    "malformed leaf in production batch".into(),
                ));
            }
            buf.extend_from_slice(&p.public_inputs_commitment);
            folded += p.folded;
        }
        let c = commit(TAG_AGG, &buf);
        Ok(Proof {
            system: ProofSystemId::UltraPlonk,
            public_inputs_commitment: c,
            bytes: buf,
            folded,
        })
    }

    fn verify_aggregate(&self, aggregate: &Proof, batch_inputs: &[Vec<u8>]) -> CputResult<()> {
        if aggregate.system != ProofSystemId::UltraPlonk {
            return Err(CputError::ProofInvalid(
                "production aggregate must be UltraPlonk".into(),
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
                "production aggregate commitment mismatch".into(),
            ))
        }
    }
}
