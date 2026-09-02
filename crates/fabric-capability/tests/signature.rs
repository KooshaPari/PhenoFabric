//! Tests: signature sign and verify roundtrip.
//!
//! Verifies that signed descriptors can be verified, and that:
//! - verification with a different key fails
//! - tampered descriptors fail verification
//! - descriptors with no signatures fail verification

use fabric_capability::descriptor::{Capabilities, CapabilityDescriptor};
use fabric_capability::{sign, verify, SigningKey};
use uuid::Uuid;

fn test_descriptor() -> CapabilityDescriptor {
    CapabilityDescriptor {
        node_id: Uuid::now_v7(),
        epoch: 1,
        schema_version: "1.0.0".to_string(),
        probed_at: chrono::Utc::now(),
        probe_version: "0.1.0".to_string(),
        topology_hash: "abc".to_string(),
        capabilities: Capabilities::default(),
        signatures: vec![],
    }
}

#[test]
fn test_sign_verify_roundtrip() {
    let key = SigningKey::generate();
    let vk = key.verification_key();
    let mut d = test_descriptor();
    sign(&mut d, &key).unwrap();

    assert_eq!(d.signatures.len(), 1);
    assert_eq!(d.signatures[0].alg, "Ed25519");

    verify(&d, &vk).expect("verify should succeed");
}

#[test]
fn test_verify_wrong_key_fails() {
    let key1 = SigningKey::generate();
    let key2 = SigningKey::generate();
    let mut d = test_descriptor();
    sign(&mut d, &key1).unwrap();

    // key1's verification key should succeed
    let vk1 = key1.verification_key();
    verify(&d, &vk1).expect("verify with key1 should succeed");

    // key2's verification key should fail
    let vk2 = key2.verification_key();
    assert!(verify(&d, &vk2).is_err());
}

#[test]
fn test_unsigned_descriptor_fails_verify() {
    let key = SigningKey::generate();
    let vk = key.verification_key();
    let d = test_descriptor();
    // No signatures → verify fails
    let result = verify(&d, &vk);
    assert!(result.is_err());
}

#[test]
fn test_tampered_descriptor_fails_verify() {
    let key = SigningKey::generate();
    let vk = key.verification_key();
    let mut d = test_descriptor();
    sign(&mut d, &key).unwrap();

    // Tamper with the descriptor (change epoch after signing)
    d.epoch = 999;

    // Verify should now fail because the signature was over the original
    // content.
    let result = verify(&d, &vk);
    assert!(result.is_err());
}

#[test]
fn test_multiple_signatures_one_trusted() {
    let key1 = SigningKey::generate();
    let key2 = SigningKey::generate();
    let mut d = test_descriptor();
    sign(&mut d, &key1).unwrap();
    sign(&mut d, &key2).unwrap();

    // Either key's verification should succeed
    let vk1 = key1.verification_key();
    let vk2 = key2.verification_key();
    verify(&d, &vk1).expect("vk1 should succeed");
    verify(&d, &vk2).expect("vk2 should succeed");
}

#[test]
fn test_key_id_matches() {
    let key = SigningKey::generate();
    let mut d = test_descriptor();
    sign(&mut d, &key).unwrap();
    assert_eq!(d.signatures[0].key_id, key.key_id());
}
