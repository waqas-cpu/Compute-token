//! Backend selection: `snark` (default) | `production` | `reference`.

use crate::production::ProductionBackend;
use crate::proof::ProofBackend;
use crate::reference::ReferenceBackend;
use crate::snark::SnarkBackend;
use std::env;
use std::fmt;

/// Selectable ZK backend wired through the pipeline.
#[derive(Debug, Clone)]
pub enum ZkBackend {
    /// SHA-256 commitment stub (integration only).
    Reference(ReferenceBackend),
    /// Witness-bound hash backend (pre-SNARK integration).
    Production(ProductionBackend),
    /// Groth16 SNARK on BN254 (production default).
    Snark(SnarkBackend),
}

impl Default for ZkBackend {
    fn default() -> Self {
        Self::from_env()
    }
}

impl ZkBackend {
    /// Read `ZK_BACKEND` (`snark` | `production` | `reference`). Defaults to `snark`.
    pub fn from_env() -> Self {
        match env::var("ZK_BACKEND")
            .unwrap_or_else(|_| "snark".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "reference" | "ref" | "stub" => Self::Reference(ReferenceBackend),
            "production" | "bound" | "hash" => Self::Production(ProductionBackend),
            _ => Self::Snark(SnarkBackend),
        }
    }

    /// Explicit backend kind for binaries and tests.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Reference(_) => "reference",
            Self::Production(_) => "production",
            Self::Snark(_) => "snark",
        }
    }
}

impl ProofBackend for ZkBackend {
    fn prove(&self, public_inputs: &[u8]) -> cput_core::CputResult<crate::proof::Proof> {
        match self {
            Self::Reference(b) => b.prove(public_inputs),
            Self::Production(b) => b.prove(public_inputs),
            Self::Snark(b) => b.prove(public_inputs),
        }
    }

    fn verify(
        &self,
        proof: &crate::proof::Proof,
        public_inputs: &[u8],
    ) -> cput_core::CputResult<()> {
        match self {
            Self::Reference(b) => b.verify(proof, public_inputs),
            Self::Production(b) => b.verify(proof, public_inputs),
            Self::Snark(b) => b.verify(proof, public_inputs),
        }
    }

    fn aggregate(&self, proofs: &[crate::proof::Proof]) -> cput_core::CputResult<crate::proof::Proof> {
        match self {
            Self::Reference(b) => b.aggregate(proofs),
            Self::Production(b) => b.aggregate(proofs),
            Self::Snark(b) => b.aggregate(proofs),
        }
    }

    fn verify_aggregate(
        &self,
        aggregate: &crate::proof::Proof,
        batch_inputs: &[Vec<u8>],
    ) -> cput_core::CputResult<()> {
        match self {
            Self::Reference(b) => b.verify_aggregate(aggregate, batch_inputs),
            Self::Production(b) => b.verify_aggregate(aggregate, batch_inputs),
            Self::Snark(b) => b.verify_aggregate(aggregate, batch_inputs),
        }
    }

    fn prove_compute(
        &self,
        statement: &crate::statements::ComputeStatement,
        witness: &crate::witness::ComputeWitness,
    ) -> cput_core::CputResult<crate::proof::Proof> {
        match self {
            Self::Reference(b) => b.prove_compute(statement, witness),
            Self::Production(b) => b.prove_compute(statement, witness),
            Self::Snark(b) => b.prove_compute(statement, witness),
        }
    }

    fn prove_reasoning(
        &self,
        statement: &crate::statements::AgentReasoningStatement,
        witness: &crate::witness::ReasoningWitness,
    ) -> cput_core::CputResult<crate::proof::Proof> {
        match self {
            Self::Reference(b) => b.prove_reasoning(statement, witness),
            Self::Production(b) => b.prove_reasoning(statement, witness),
            Self::Snark(b) => b.prove_reasoning(statement, witness),
        }
    }
}

impl fmt::Display for ZkBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ZkBackend({})", self.kind())
    }
}
