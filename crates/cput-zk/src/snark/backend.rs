//! Groth16 SNARK backend — real zero-knowledge proofs for L0/L1/L2.

use crate::proof::{Proof, ProofBackend, ProofSystemId};
use crate::snark::circuits::{ComputeCircuit, ReasoningCircuit};
use crate::snark::field::{u128_to_fr, workload_binding_fr};
use crate::snark::codec::{
    aggregate_commitment, deserialize_groth16, pack_batch, public_commitment,
    serialize_groth16, unpack_batch, verify_groth16,
};
use crate::snark::keys::{compute_keys, reasoning_keys};
use crate::statements::{AgentReasoningStatement, ComputeStatement};
use crate::witness::{ComputeWitness, ReasoningWitness};
use ark_bn254::Bn254;
use ark_groth16::Groth16;
use ark_snark::SNARK;
use ark_std::rand::{rngs::StdRng, SeedableRng};
use cput_core::{CputError, CputResult};

/// Groth16 (BN254) SNARK backend used by the production pipeline.
#[derive(Debug, Clone, Default)]
pub struct SnarkBackend;

impl SnarkBackend {
    fn prove_compute_circuit(
        statement: &ComputeStatement,
        witness: &ComputeWitness,
    ) -> CputResult<Proof> {
        Self::validate_compute(statement, witness)?;
        let keys = compute_keys()?;
        let circuit = ComputeCircuit::from_statement(statement, witness);
        let public = circuit.public_inputs();
        let public_bytes = crate::encode_statement(statement)?;
        let mut rng = StdRng::seed_from_u64(statement.epoch.0 ^ statement.node_id.0[0] as u64);
        let groth16 = Groth16::<Bn254>::prove(&keys.pk, circuit, &mut rng)
            .map_err(|e| CputError::ProofInvalid(format!("compute prove: {e}")))?;
        let bytes = serialize_groth16(&groth16)?;
        verify_groth16(&keys.vk, &public, &groth16)?;
        Ok(Proof {
            system: ProofSystemId::Groth16,
            public_inputs_commitment: public_commitment(&public_bytes),
            bytes,
            folded: 1,
        })
    }

    fn prove_reasoning_circuit(
        statement: &AgentReasoningStatement,
        witness: &ReasoningWitness,
    ) -> CputResult<Proof> {
        Self::validate_reasoning(statement, witness)?;
        let keys = reasoning_keys()?;
        let circuit = ReasoningCircuit::from_statement(statement, witness);
        let public = circuit.public_inputs();
        let public_bytes = crate::encode_statement(statement)?;
        let mut rng = StdRng::seed_from_u64(statement.epoch.0);
        let groth16 = Groth16::<Bn254>::prove(&keys.pk, circuit, &mut rng)
            .map_err(|e| CputError::ProofInvalid(format!("reasoning prove: {e}")))?;
        let bytes = serialize_groth16(&groth16)?;
        verify_groth16(&keys.vk, &public, &groth16)?;
        Ok(Proof {
            system: ProofSystemId::Groth16,
            public_inputs_commitment: public_commitment(&public_bytes),
            bytes,
            folded: 1,
        })
    }

    fn verify_leaf(proof: &Proof, public_inputs: &[u8]) -> CputResult<()> {
        if proof.system != ProofSystemId::Groth16 {
            return Err(CputError::ProofInvalid(
                "expected Groth16 proof system".into(),
            ));
        }
        if public_commitment(public_inputs) != proof.public_inputs_commitment {
            return Err(CputError::ProofInvalid(
                "public inputs commitment mismatch".into(),
            ));
        }
        let groth16 = deserialize_groth16(&proof.bytes)?;
        if let Ok(stmt) = serde_json::from_slice::<ComputeStatement>(public_inputs) {
            let public = vec![
                u128_to_fr(stmt.gflops.0),
                workload_binding_fr(stmt.workload_hash.0),
            ];
            return verify_groth16(&compute_keys()?.vk, &public, &groth16);
        }
        if let Ok(stmt) = serde_json::from_slice::<AgentReasoningStatement>(public_inputs) {
            let public = vec![u128_to_fr(stmt.mint_amount)];
            return verify_groth16(&reasoning_keys()?.vk, &public, &groth16);
        }
        Err(CputError::ProofInvalid(
            "unsupported statement type for groth16 verify".into(),
        ))
    }

    fn validate_compute(statement: &ComputeStatement, witness: &ComputeWitness) -> CputResult<()> {
        if witness.measured_gflops != statement.gflops.0 {
            return Err(CputError::ProofInvalid(
                "witness gflops != public gflops".into(),
            ));
        }
        let expected = crate::snark::field::workload_binding_fr(statement.workload_hash.0);
        if crate::snark::field::bytes32_to_fr(witness.execution_trace_root) != expected {
            return Err(CputError::ProofInvalid(
                "trace root does not bind workload".into(),
            ));
        }
        Ok(())
    }

    fn validate_reasoning(
        statement: &AgentReasoningStatement,
        witness: &ReasoningWitness,
    ) -> CputResult<()> {
        if witness.computed_mint != statement.mint_amount {
            return Err(CputError::ProofInvalid(
                "witness mint != public mint".into(),
            ));
        }
        Ok(())
    }
}

impl ProofBackend for SnarkBackend {
    fn prove_compute(
        &self,
        statement: &ComputeStatement,
        witness: &ComputeWitness,
    ) -> CputResult<Proof> {
        Self::prove_compute_circuit(statement, witness)
    }

    fn prove_reasoning(
        &self,
        statement: &AgentReasoningStatement,
        witness: &ReasoningWitness,
    ) -> CputResult<Proof> {
        Self::prove_reasoning_circuit(statement, witness)
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
            "groth16 prove requires compute or reasoning statement JSON".into(),
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
        let mut leaf_bytes = Vec::with_capacity(proofs.len());
        let mut commitments = Vec::with_capacity(proofs.len());
        let mut folded = 0u32;
        for p in proofs {
            if p.system != ProofSystemId::Groth16 {
                return Err(CputError::ProofInvalid(
                    "aggregate requires Groth16 leaf proofs".into(),
                ));
            }
            leaf_bytes.push(p.bytes.clone());
            commitments.push(p.public_inputs_commitment);
            folded += p.folded;
        }
        Ok(Proof {
            system: ProofSystemId::Groth16,
            public_inputs_commitment: aggregate_commitment(&commitments),
            bytes: pack_batch(&leaf_bytes)?,
            folded,
        })
    }

    fn verify_aggregate(&self, aggregate: &Proof, batch_inputs: &[Vec<u8>]) -> CputResult<()> {
        if aggregate.system != ProofSystemId::Groth16 {
            return Err(CputError::ProofInvalid(
                "aggregate must be Groth16".into(),
            ));
        }
        let leaves = unpack_batch(&aggregate.bytes)?;
        if leaves.len() != batch_inputs.len() {
            return Err(CputError::ProofInvalid(format!(
                "batch size {} != inputs {}",
                leaves.len(),
                batch_inputs.len()
            )));
        }
        let mut commitments = Vec::with_capacity(leaves.len());
        for (leaf_bytes, inputs) in leaves.iter().zip(batch_inputs) {
            let proof = Proof {
                system: ProofSystemId::Groth16,
                public_inputs_commitment: public_commitment(inputs),
                bytes: leaf_bytes.clone(),
                folded: 1,
            };
            self.verify(&proof, inputs)?;
            commitments.push(proof.public_inputs_commitment);
        }
        if aggregate.public_inputs_commitment != aggregate_commitment(&commitments) {
            return Err(CputError::ProofInvalid(
                "aggregate commitment mismatch".into(),
            ));
        }
        Ok(())
    }
}
