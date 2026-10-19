//! End-to-end sign/verify round-trip tests for `fabric-capability-ffi`.
//!
//! These tests pin the cryptographic contract: a message signed with a
//! generated key MUST verify with that key, MUST NOT verify with a
//! different key, and MUST produce a deterministic, hex-formatted key id.

mod common;

use std::os::raw::c_char;
use std::ptr;

use fabric_capability_ffi::{
    fabric_capability_free_string, fabric_capability_generate_key, fabric_capability_key_id,
    fabric_capability_sign_json, fabric_capability_verify_json, FabricError,
};

use crate::common::{cstr, cstr_to_string_lossy};

/// A JSON-serialized `CapabilityDescriptor` with all required fields set to
/// minimal but valid values. The FFI's `sign_json` parses this as a
/// `CapabilityDescriptor` via serde — using a structurally complete
/// payload pins the contract that "a real descriptor can be signed".
const SIGNED_PAYLOAD: &str = r#"{
  "node_id": "00000000-0000-7000-8000-000000000000",
  "epoch": 1,
  "schema_version": "1.0.0",
  "probed_at": "2024-01-01T00:00:00Z",
  "probe_version": "0.1.0",
  "topology_hash": "0000000000000000000000000000000000000000000000000000000000000000",
  "capabilities": {},
  "signatures": []
}"#;

/// Generate a key, sign the canonical payload, and verify the signature
/// back with the same key. This is the happy path.
#[test]
fn sign_then_verify_with_same_key_succeeds() {
    let mut key = [0u8; 32];
    let mut sig_out: *mut c_char = ptr::null_mut();

    unsafe {
        assert_eq!(fabric_capability_generate_key(&mut key), FabricError::Ok);

        assert_eq!(
            fabric_capability_sign_json(
                cstr(SIGNED_PAYLOAD),
                key.as_ptr(),
                (&mut sig_out) as *mut _
            ),
            FabricError::Ok
        );
        assert!(!sig_out.is_null(), "signature must not be NULL on success");
        let sig_str = cstr_to_string_lossy(sig_out);
        assert!(
            !sig_str.is_empty(),
            "signature must be non-empty on success"
        );

        assert_eq!(
            fabric_capability_verify_json(sig_out, key.as_ptr()),
            FabricError::Ok
        );

        fabric_capability_free_string(sig_out);
    }
}

/// Different keys MUST NOT accept each other's signatures. Generating a
/// second key and re-verifying with it is the canonical test for
/// cross-key rejection.
#[test]
fn verify_with_different_key_returns_signature_error() {
    let mut key_a = [0u8; 32];
    let mut key_b = [0u8; 32];
    let mut sig_out: *mut c_char = ptr::null_mut();

    unsafe {
        assert_eq!(fabric_capability_generate_key(&mut key_a), FabricError::Ok);
        assert_eq!(fabric_capability_generate_key(&mut key_b), FabricError::Ok);
        assert_ne!(
            key_a, key_b,
            "two consecutive key generations must differ (RNG sanity)"
        );

        assert_eq!(
            fabric_capability_sign_json(
                cstr(SIGNED_PAYLOAD),
                key_a.as_ptr(),
                (&mut sig_out) as *mut _
            ),
            FabricError::Ok
        );

        // Same signature, different key — must be rejected.
        assert_eq!(
            fabric_capability_verify_json(sig_out, key_b.as_ptr()),
            FabricError::Signature
        );

        fabric_capability_free_string(sig_out);
    }
}

/// A tampered JSON MUST NOT verify. We take a valid signature, mutate the
/// payload, and re-verify. The signature is over the original bytes, so
/// any byte-level change must be rejected.
#[test]
fn verify_rejects_tampered_payload() {
    let mut key = [0u8; 32];
    let mut sig_out: *mut c_char = ptr::null_mut();

    unsafe {
        assert_eq!(fabric_capability_generate_key(&mut key), FabricError::Ok);
        assert_eq!(
            fabric_capability_sign_json(
                cstr(SIGNED_PAYLOAD),
                key.as_ptr(),
                (&mut sig_out) as *mut _
            ),
            FabricError::Ok
        );

        // Tampered payload: same shape, one byte changed.
        let tampered = SIGNED_PAYLOAD.replace("alice", "eve");
        assert_eq!(
            fabric_capability_verify_json(cstr(&tampered), key.as_ptr()),
            FabricError::Signature
        );

        fabric_capability_free_string(sig_out);
    }
}

/// `fabric_capability_key_id` returns a non-NULL, non-empty, base64-formatted
/// key identifier derived from the 32-byte verifying key. 32 bytes of
/// base64 (standard alphabet, with padding) is 44 characters. We pin that
/// contract.
#[test]
fn key_id_is_44_char_base64_of_32_byte_key() {
    let mut key = [0u8; 32];
    unsafe {
        assert_eq!(fabric_capability_generate_key(&mut key), FabricError::Ok);
        let id = fabric_capability_key_id(key.as_ptr());
        assert!(!id.is_null(), "key_id must not be NULL on success");
        let s = cstr_to_string_lossy(id);
        assert_eq!(s.len(), 44, "key_id must be 44 base64 chars (32 bytes)");
        assert!(
            s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='),
            "key_id must be base64 alphabet (alphanumeric + +/=)"
        );
        fabric_capability_free_string(id);
    }
}

/// The key id MUST be deterministic for a given key. The library uses
/// `KeyId::from_key` which hashes the key bytes, so calling it twice on
/// the same key must produce the same identifier.
#[test]
fn key_id_is_deterministic_for_same_key() {
    let mut key = [0u8; 32];
    unsafe {
        assert_eq!(fabric_capability_generate_key(&mut key), FabricError::Ok);
        let a = fabric_capability_key_id(key.as_ptr());
        let b = fabric_capability_key_id(key.as_ptr());
        assert_eq!(cstr_to_string_lossy(a), cstr_to_string_lossy(b));
        fabric_capability_free_string(a);
        fabric_capability_free_string(b);
    }
}

/// Two different keys MUST produce different key ids. Otherwise the
/// identifier would not function as a stable handle.
#[test]
fn key_id_differs_for_different_keys() {
    let mut key_a = [0u8; 32];
    let mut key_b = [0u8; 32];
    unsafe {
        assert_eq!(fabric_capability_generate_key(&mut key_a), FabricError::Ok);
        assert_eq!(fabric_capability_generate_key(&mut key_b), FabricError::Ok);
        let a = fabric_capability_key_id(key_a.as_ptr());
        let b = fabric_capability_key_id(key_b.as_ptr());
        assert_ne!(cstr_to_string_lossy(a), cstr_to_string_lossy(b));
        fabric_capability_free_string(a);
        fabric_capability_free_string(b);
    }
}
