//! Integration tests: real NIST PQC sign/verify and sealed envelopes.

use cput_pqc::envelope::{SealedEnvelope, Signed};
use cput_pqc::mlkem::{encapsulate_to, MlKemKeypair};
use cput_pqc::mldsa::{self, MlDsaKeypair};
use cput_pqc::registry::{AlgorithmId, AlgorithmRegistry};
use cput_pqc::slhdsa::{self, SlhDsaKeypair};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Payload {
    epoch: u64,
    gflops: u128,
}

#[test]
fn mldsa_sign_verify_roundtrip() {
    let kp = MlDsaKeypair::generate().expect("keygen");
    let msg = b"attestation-body-v1";
    let sig = kp.sign(msg).expect("sign");
    assert!(sig.len() > 1000, "ML-DSA-87 signatures are kilobyte-scale");
    mldsa::verify(&kp.public_bytes(), msg, &sig).expect("verify");
}

#[test]
fn mldsa_rejects_tampered_signature() {
    let kp = MlDsaKeypair::generate().expect("keygen");
    let msg = b"epoch-report";
    let mut sig = kp.sign(msg).expect("sign");
    sig[0] ^= 0xFF;
    assert!(mldsa::verify(&kp.public_bytes(), msg, &sig).is_err());
}

#[test]
fn slhdsa_sign_verify_roundtrip() {
    let kp = SlhDsaKeypair::generate().expect("keygen");
    let msg = b"audit-merkle-root";
    let sig = kp.sign(msg).expect("sign");
    assert!(sig.len() > 4000, "SLH-DSA-256s signatures are multi-kilobyte");
    slhdsa::verify(&kp.public_bytes(), msg, &sig).expect("verify");
}

#[test]
fn sealed_envelope_mldsa_admits_at_gate() {
    let registry = AlgorithmRegistry::default();
    let kp = MlDsaKeypair::generate().expect("keygen");
    let payload = b"canonical-attestation-payload".to_vec();
    let env = SealedEnvelope::seal_mldsa(&kp, payload).expect("seal");
    assert_eq!(env.algorithm, AlgorithmId::MlDsa87);
    assert!(!env.signature.is_empty());
    env.verify(&registry).expect("envelope verify");
}

#[test]
fn mlkem_encaps_decaps_shared_secret() {
    let alice = MlKemKeypair::generate().expect("keygen");
    let bob = MlKemKeypair::generate().expect("keygen");
    let ct = encapsulate_to(&bob.encaps_public_bytes()).expect("encaps");
    let bob_secret = bob.decapsulate(&ct.ciphertext).expect("decaps");
    assert_eq!(ct.shared_secret, bob_secret);
    assert_ne!(ct.shared_secret, [0u8; 32]);
}

#[test]
fn signed_body_binds_payload_to_signature() {
    let registry = AlgorithmRegistry::default();
    let kp = MlDsaKeypair::generate().expect("keygen");
    let body = Payload {
        epoch: 1007,
        gflops: 37_750_050_000,
    };
    let signed = Signed::seal_mldsa(&kp, body.clone()).expect("seal");
    signed.verify(&registry).expect("signed verify");

    let mut bad = signed.clone();
    bad.body.gflops += 1;
    assert!(bad.verify(&registry).is_err(), "tampered body must fail");
}
