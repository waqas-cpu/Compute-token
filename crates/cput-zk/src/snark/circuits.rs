//! Groth16 R1CS circuits for compute (L0) and reasoning (L2) statements.

use crate::snark::field::{bytes32_to_fr, u128_to_fr, workload_binding_fr};
use crate::statements::{AgentReasoningStatement, ComputeStatement};
use crate::witness::{ComputeWitness, ReasoningWitness};
use ark_bn254::Fr;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;

/// Groth16 circuit: measured GFLOPs and trace root match public attestation.
#[derive(Clone, Debug)]
pub struct ComputeCircuit {
    /// Public attested GFLOPs.
    pub gflops_pub: Option<Fr>,
    /// Public workload binding (derived from `workload_hash`).
    pub workload_pub: Option<Fr>,
    /// Private measured GFLOPs.
    pub measured_gflops: Option<Fr>,
    /// Private execution trace root.
    pub trace_root: Option<Fr>,
}

impl ComputeCircuit {
    /// Instantiate from a statement + witness pair.
    pub fn from_statement(statement: &ComputeStatement, witness: &ComputeWitness) -> Self {
        Self {
            gflops_pub: Some(u128_to_fr(statement.gflops.0)),
            workload_pub: Some(workload_binding_fr(statement.workload_hash.0)),
            measured_gflops: Some(u128_to_fr(witness.measured_gflops)),
            trace_root: Some(bytes32_to_fr(witness.execution_trace_root)),
        }
    }

    /// Blank circuit for trusted-setup key generation.
    #[must_use]
    pub fn blank() -> Self {
        Self {
            gflops_pub: None,
            workload_pub: None,
            measured_gflops: None,
            trace_root: None,
        }
    }

    /// Public inputs in verifier order.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![
            self.gflops_pub.expect("gflops_pub"),
            self.workload_pub.expect("workload_pub"),
        ]
    }
}

impl ConstraintSynthesizer<Fr> for ComputeCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let gflops_pub = FpVar::new_input(cs.clone(), || {
            self.gflops_pub.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let workload_pub = FpVar::new_input(cs.clone(), || {
            self.workload_pub.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let measured = FpVar::new_witness(cs.clone(), || {
            self.measured_gflops.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let trace_root = FpVar::new_witness(cs.clone(), || {
            self.trace_root.ok_or(SynthesisError::AssignmentMissing)
        })?;

        measured.enforce_equal(&gflops_pub)?;
        trace_root.enforce_equal(&workload_pub)?;
        Ok(())
    }
}

/// Groth16 circuit: agent formula output matches public mint amount.
#[derive(Clone, Debug)]
pub struct ReasoningCircuit {
    /// Public mint total.
    pub mint_pub: Option<Fr>,
    /// Private computed mint.
    pub computed_mint: Option<Fr>,
}

impl ReasoningCircuit {
    /// Instantiate from a statement + witness pair.
    pub fn from_statement(
        statement: &AgentReasoningStatement,
        witness: &ReasoningWitness,
    ) -> Self {
        Self {
            mint_pub: Some(u128_to_fr(statement.mint_amount)),
            computed_mint: Some(u128_to_fr(witness.computed_mint)),
        }
    }

    /// Blank circuit for trusted-setup key generation.
    #[must_use]
    pub fn blank() -> Self {
        Self {
            mint_pub: None,
            computed_mint: None,
        }
    }

    /// Public inputs in verifier order.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![self.mint_pub.expect("mint_pub")]
    }
}

impl ConstraintSynthesizer<Fr> for ReasoningCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let mint_pub = FpVar::new_input(cs.clone(), || {
            self.mint_pub.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let computed = FpVar::new_witness(cs.clone(), || {
            self.computed_mint.ok_or(SynthesisError::AssignmentMissing)
        })?;
        computed.enforce_equal(&mint_pub)?;
        Ok(())
    }
}
