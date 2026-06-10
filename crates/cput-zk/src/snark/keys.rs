//! Circuit-specific Groth16 proving / verifying keys (lazy one-time setup).

use crate::snark::circuits::{ComputeCircuit, ReasoningCircuit};
use ark_bn254::Bn254;
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_snark::SNARK;
use ark_std::rand::{rngs::StdRng, SeedableRng};
use cput_core::{CputError, CputResult};
use std::sync::OnceLock;

/// Groth16 key material for a single circuit type.
pub struct CircuitKeys {
    /// Proving key.
    pub pk: ProvingKey<Bn254>,
    /// Verifying key.
    pub vk: VerifyingKey<Bn254>,
}

fn setup_compute() -> CputResult<CircuitKeys> {
    let mut rng = StdRng::seed_from_u64(0x4350_5554_5f43_4f4d); // "CPUT_COM"
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(ComputeCircuit::blank(), &mut rng)
        .map_err(|e| CputError::ProofInvalid(format!("compute setup: {e}")))?;
    Ok(CircuitKeys { pk, vk })
}

fn setup_reasoning() -> CputResult<CircuitKeys> {
    let mut rng = StdRng::seed_from_u64(0x4350_5554_5f5245); // "CPUT_RE"
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(ReasoningCircuit::blank(), &mut rng)
        .map_err(|e| CputError::ProofInvalid(format!("reasoning setup: {e}")))?;
    Ok(CircuitKeys { pk, vk })
}

static COMPUTE_KEYS: OnceLock<CircuitKeys> = OnceLock::new();
static REASONING_KEYS: OnceLock<CircuitKeys> = OnceLock::new();

/// Lazily initialized compute-circuit keys.
pub fn compute_keys() -> CputResult<&'static CircuitKeys> {
    if let Some(keys) = COMPUTE_KEYS.get() {
        return Ok(keys);
    }
    let keys = setup_compute()?;
    let _ = COMPUTE_KEYS.set(keys);
    Ok(COMPUTE_KEYS.get().expect("compute keys set"))
}

/// Lazily initialized reasoning-circuit keys.
pub fn reasoning_keys() -> CputResult<&'static CircuitKeys> {
    if let Some(keys) = REASONING_KEYS.get() {
        return Ok(keys);
    }
    let keys = setup_reasoning()?;
    let _ = REASONING_KEYS.set(keys);
    Ok(REASONING_KEYS.get().expect("reasoning keys set"))
}
