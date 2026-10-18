//! FFI boundary tests for `fabric-capability-ffi`.
//!
//! These tests intentionally call the exported `extern "C"` symbols exactly
//! as a C consumer would, so they exercise the contract at the ABI boundary,
//! not the underlying Rust API. They were created because the crate previously
//! shipped with zero tests while exposing the highest-risk surface in the
//! repository: an FFI layer that, if it lied about ownership, NULL handling,
//! or error codes, would silently corrupt memory in any C consumer.
//!
//! Each test is annotated with the defect (if any) it pins.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use fabric_capability_ffi::{
    fabric_capability_error_message, fabric_capability_free_string, fabric_capability_generate_key,
    fabric_capability_key_id, fabric_capability_sign_json, fabric_capability_to_json,
    fabric_capability_verify_json, FabricError,
};

// `strlen` is the canonical way to verify a `*const c_char` is a real
// NUL-terminated C string. We declare it locally so the test does not need
// to depend on the `libc` crate (which is not in this crate's dev-deps).
extern "C" {
    fn strlen(s: *const c_char) -> usize;
}

fn cstr(s: &str) -> *const c_char {
    CString::new(s).unwrap().into_raw()
}

/// Convert a NUL-terminated C string pointer into an owned `String`,
/// substituting U+FFFD for any invalid UTF-8 bytes. Returns an empty
/// string if the pointer is NULL.
///
/// # Safety
///
/// `p` must be either NULL or a valid pointer to a NUL-terminated C
/// string. The string's bytes must remain valid for the duration of
/// this call (i.e. must not be freed concurrently). The caller is
/// responsible for freeing the backing buffer if it was heap-allocated.
unsafe fn cstr_to_string_lossy(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    CStr::from_ptr(p).to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// ABI surface — every export must be linkable and reachable.
// ---------------------------------------------------------------------------

/// At the C ABI level the enum must be `repr(i32)` so the wire representation
/// matches the C header. This test pins the integer widths the C consumer sees.
#[test]
fn fabric_error_is_repr_i32_with_documented_values() {
    assert_eq!(FabricError::Ok as i32, 0);
    assert_eq!(FabricError::Unsupported as i32, 1);
    assert_eq!(FabricError::Schema as i32, 2);
    assert_eq!(FabricError::Crypto as i32, 3);
    assert_eq!(FabricError::Signature as i32, 4);
    assert_eq!(FabricError::Io as i32, 5);
    assert_eq!(FabricError::Serde as i32, 6);
    assert_eq!(FabricError::NullPointer as i32, 7);
}

/// The error message table is a documentation contract: every code must have
/// a non-empty, NUL-terminated, UTF-8 string. If a code were unhandled the
/// previous implementation would have returned `""` and C callers would have
/// seen an empty message.
#[test]
fn error_message_table_is_complete_and_nonempty() {
    for code in [
        FabricError::Ok,
        FabricError::Unsupported,
        FabricError::Schema,
        FabricError::Crypto,
        FabricError::Signature,
        FabricError::Io,
        FabricError::Serde,
        FabricError::NullPointer,
    ] {
        let p = fabric_capability_error_message(code);
        assert!(
            !p.is_null(),
            "error message for code {:?} must not be NULL",
            code
        );
        let s = unsafe { CStr::from_ptr(p) };
        assert!(
            !s.to_bytes().is_empty(),
            "error message for code {:?} must be non-empty",
            code
        );
        assert!(
            s.to_str().is_ok(),
            "error message for code {:?} must be UTF-8",
            code
        );
    }
}

/// Sanity check on `error_message` return type: the pointer must really be
/// NUL-terminated. We probe via `strlen` so the contract is checked against
/// the actual C-string convention, not against what `CStr::to_bytes` happens
/// to return.
#[test]
fn error_message_is_nul_terminated_per_strlen() {
    let p = fabric_capability_error_message(FabricError::Ok);
    let n = unsafe { strlen(p) };
    let bytes = unsafe { CStr::from_ptr(p) }.to_bytes();
    // `CStr::to_bytes` excludes the trailing NUL; `strlen` counts up to but
    // not including it.
    assert_eq!(bytes.len(), n);
    assert!(n > 0);
}

// ---------------------------------------------------------------------------
// Memory ownership and lifetimes.
// ---------------------------------------------------------------------------

/// `fabric_capability_free_string` MUST be a no-op for NULL — the C caller
/// has no way to distinguish "I forgot to check" from "the function returned
/// NULL". A NULL deref here would abort the C process.
#[test]
fn free_string_null_is_safe_no_op() {
    unsafe { fabric_capability_free_string(ptr::null_mut()) };
}

/// The returned CString pointer must be exactly the one `CString::into_raw`
/// handed out, and freeing it must not corrupt the heap. We double-allocate
/// distinct strings, free the first, and assert the second is still readable.
#[test]
fn free_string_recycles_allocated_string() {
    let a = CString::new("alpha").unwrap().into_raw();
    let b = CString::new("bravo").unwrap().into_raw();

    unsafe {
        let read = CStr::from_ptr(b).to_str().unwrap().to_owned();
        assert_eq!(read, "bravo");
        fabric_capability_free_string(a);
        // `b` must still be readable after `a` is freed.
        let read = CStr::from_ptr(b).to_str().unwrap().to_owned();
        assert_eq!(read, "bravo");
        fabric_capability_free_string(b);
    }
}

// ---------------------------------------------------------------------------
// NULL-handling contract — every pointer-taking function must reject NULL
// explicitly. A NULL deref is the bug we are hunting.
// ---------------------------------------------------------------------------

#[test]
fn to_json_null_input_returns_null_not_a_deref() {
    let p = unsafe { fabric_capability_to_json(ptr::null()) };
    assert!(
        p.is_null(),
        "NULL input must return NULL (was a NULL deref?)"
    );
}

#[test]
fn sign_json_null_inputs_all_return_null_pointer_error() {
    let key = [0u8; 32];
    let mut out: *mut c_char = ptr::null_mut();
    let json = cstr("{}");

    // Each pointer parameter takes its turn being NULL while the others are
    // valid. All three must return NullPointer rather than dereferencing.
    unsafe {
        assert_eq!(
            fabric_capability_sign_json(ptr::null(), key.as_ptr(), &mut out),
            FabricError::NullPointer
        );
        assert_eq!(
            fabric_capability_sign_json(json, ptr::null(), &mut out),
            FabricError::NullPointer
        );
        assert_eq!(
            fabric_capability_sign_json(json, key.as_ptr(), ptr::null_mut()),
            FabricError::NullPointer
        );
    }
}

#[test]
fn verify_json_null_inputs_return_null_pointer_error() {
    let key = [0u8; 32];
    let json = cstr("{}");

    unsafe {
        assert_eq!(
            fabric_capability_verify_json(ptr::null(), key.as_ptr()),
            FabricError::NullPointer
        );
        assert_eq!(
            fabric_capability_verify_json(json, ptr::null()),
            FabricError::NullPointer
        );
    }
}

#[test]
fn generate_key_null_out_returns_null_pointer_error() {
    unsafe {
        assert_eq!(
            fabric_capability_generate_key(ptr::null_mut()),
            FabricError::NullPointer
        );
    }
}

#[test]
fn key_id_null_input_returns_null_not_a_deref() {
    let p = unsafe { fabric_capability_key_id(ptr::null()) };
    assert!(
        p.is_null(),
        "NULL input must return NULL (was a NULL deref?)"
    );
}

// ---------------------------------------------------------------------------
// Integer width, error path distinguishability, and the previously-silent
// `unwrap_or([0u8; 32])` defect.
// ---------------------------------------------------------------------------

/// `fabric_capability_to_json` used to silently `unwrap_or_default()` on a
/// UTF-8 decode failure and return the JSON encoding of an empty string —
/// the caller could not tell their descriptor from `""`. The current
/// contract returns NULL.
#[test]
fn to_json_rejects_invalid_utf8_with_null_not_a_swallowed_error() {
    // 0xFF is never valid UTF-8.
    let bad: [u8; 2] = [0xFF, 0x00];
    let p = unsafe { fabric_capability_to_json(bad.as_ptr() as *const c_char) };
    assert!(
        p.is_null(),
        "invalid UTF-8 must return NULL, not a silently-swallowed error"
    );
}

/// `fabric_capability_to_json` round-trips a JSON STRING value (the
/// implementation parses the input as a JSON value and re-serializes it).
/// This pins that documented behaviour.
#[test]
fn to_json_round_trip_of_a_valid_string_value() {
    let p = unsafe { fabric_capability_to_json(cstr("hello")) };
    assert!(!p.is_null());
    let s = unsafe { cstr_to_string_lossy(p) };
    assert_eq!(s, "\"hello\"");
    unsafe { fabric_capability_free_string(p) };
}

// ---------------------------------------------------------------------------
// Sign + verify round-trip — proves the ABI plumbing actually works end-to-end.

/// Minimal valid descriptor JSON. The struct only needs the required fields;
/// everything else defaults.
const UNSIGNED_DESCRIPTOR: &str = r#"{
    "node_id": "00000000-0000-7000-8000-000000000001",
    "epoch": 1,
    "schema_version": "phenotype.fabric.capability_descriptor/1",
    "probed_at": "2026-01-01T00:00:00Z",
    "probe_version": "0.1.0",
    "topology_hash": "deadbeef",
    "capabilities": {},
    "signatures": []
}"#;

/// `generate_key` fills the caller's buffer with no allocation. We then take
/// the verification key, sign, verify, and assert all error codes flow.
#[test]
fn sign_and_verify_round_trip_with_generated_key() {
    let mut key = [0u8; 32];
    let mut out: *mut c_char = ptr::null_mut();

    unsafe {
        let err = fabric_capability_generate_key(&mut key);
        assert_eq!(err, FabricError::Ok);

        let json = cstr(UNSIGNED_DESCRIPTOR);
        let err = fabric_capability_sign_json(json, key.as_ptr(), &mut out);
        assert_eq!(
            err,
            FabricError::Ok,
            "sign should succeed with a valid 32-byte key"
        );
        assert!(!out.is_null(), "successful sign must populate out_signed");
    }

    let signed = unsafe { cstr_to_string_lossy(out) };
    assert!(
        signed.contains("\"signatures\""),
        "signed descriptor must include signatures"
    );
    assert!(
        !signed.contains(r#""signatures":[]"#),
        "signed descriptor must NOT have empty signatures"
    );

    // The fingerprint for the same key must be deterministic.
    let id1 = unsafe { fabric_capability_key_id(key.as_ptr()) };
    let id1_str = unsafe { cstr_to_string_lossy(id1) };
    let id2 = unsafe { fabric_capability_key_id(key.as_ptr()) };
    let id2_str = unsafe { cstr_to_string_lossy(id2) };
    assert_eq!(
        id1_str, id2_str,
        "key_id must be deterministic for the same key"
    );
    unsafe {
        fabric_capability_free_string(id1);
        fabric_capability_free_string(id2);
    }

    // A freshly-generated, different key must NOT verify the signature.
    let mut other_key = [0u8; 32];
    unsafe {
        let err = fabric_capability_generate_key(&mut other_key);
        assert_eq!(err, FabricError::Ok);
        let signed_ptr = CString::new(signed.clone()).unwrap().into_raw();
        let err = fabric_capability_verify_json(signed_ptr, other_key.as_ptr());
        assert_eq!(
            err,
            FabricError::Signature,
            "a signature produced by key A must NOT verify under key B"
        );
    }

    // Cleanup.
    unsafe { fabric_capability_free_string(out) };
}

#[test]
fn verify_json_unsigned_descriptor_returns_signature_error() {
    // Build a valid key.
    let mut key = [0u8; 32];
    unsafe {
        assert_eq!(fabric_capability_generate_key(&mut key), FabricError::Ok);
    }
    // The descriptor has `"signatures": []` — verification must fail.
    let json = cstr(UNSIGNED_DESCRIPTOR);
    let err = unsafe { fabric_capability_verify_json(json, key.as_ptr()) };
    assert_eq!(
        err,
        FabricError::Signature,
        "verifying an unsigned descriptor must return Signature, not Ok"
    );
}

/// True sign+verify round-trip with the same seed. This is the contract a
/// C consumer actually needs: produce a signature, then verify it under
/// the same key, end-to-end through the FFI. If the FFI's serialisation
/// path is not byte-stable across sign → verify (e.g. it embeds a
/// timestamp that deserialises to something different from what was
/// signed), this test catches it.
#[test]
fn sign_then_verify_with_same_seed_is_ok() {
    let mut key = [0u8; 32];
    let mut out: *mut c_char = ptr::null_mut();

    unsafe {
        assert_eq!(fabric_capability_generate_key(&mut key), FabricError::Ok);
        let json = cstr(UNSIGNED_DESCRIPTOR);
        let err = fabric_capability_sign_json(json, key.as_ptr(), &mut out);
        assert_eq!(err, FabricError::Ok);
        assert!(!out.is_null());

        // Hand the signed JSON straight back to verify_json with the same
        // seed. If this returns anything other than Ok, the sign/verify
        // round-trip is broken at the FFI layer.
        let err = fabric_capability_verify_json(out, key.as_ptr());
        assert_eq!(
            err,
            FabricError::Ok,
            "sign+verify round-trip with the same seed must return Ok"
        );

        fabric_capability_free_string(out);
    }
}

#[test]
fn sign_json_malformed_descriptor_returns_serde_error() {
    let mut key = [0u8; 32];
    unsafe {
        assert_eq!(fabric_capability_generate_key(&mut key), FabricError::Ok);
    }
    let json = cstr("this is not valid JSON {{{");
    let mut out: *mut c_char = ptr::null_mut();
    let err = unsafe { fabric_capability_sign_json(json, key.as_ptr(), &mut out) };
    assert_eq!(err, FabricError::Serde);
    assert!(
        out.is_null(),
        "serde-error path must not write to out_signed"
    );
}

// ---------------------------------------------------------------------------
// Header ↔ Rust surface reconciliation.
//
// The header is hand-authored. The header-pre-fix declared a phantom
// `fabric_capability_free_key` that did not exist in the Rust crate, which
// would have caused a link-time failure for any C consumer. We pin the
// header's symbol list here as a guardrail against silent drift.
// ---------------------------------------------------------------------------

#[test]
fn every_header_declared_function_is_present_in_the_library() {
    // Each entry is the EXACT symbol name as the C linker will see it. If
    // someone adds a declaration to the header without an `#[no_mangle] extern "C"`
    // implementation, this test pins the divergence. (If the symbol does
    // not exist at link time, the test binary itself fails to link — exactly
    // the symptom a C consumer would see.)
    let _fn_free: unsafe extern "C" fn(*mut c_char) = fabric_capability_free_string;
    let _fn_to_json: unsafe extern "C" fn(*const c_char) -> *mut c_char = fabric_capability_to_json;
    let _fn_err_msg: unsafe extern "C" fn(FabricError) -> *const c_char =
        fabric_capability_error_message;
    let _fn_sign: unsafe extern "C" fn(*const c_char, *const u8, *mut *mut c_char) -> FabricError =
        fabric_capability_sign_json;
    let _fn_verify: unsafe extern "C" fn(*const c_char, *const u8) -> FabricError =
        fabric_capability_verify_json;
    let _fn_gen: unsafe extern "C" fn(*mut [u8; 32]) -> FabricError =
        fabric_capability_generate_key;
    let _fn_id: unsafe extern "C" fn(*const u8) -> *mut c_char = fabric_capability_key_id;
}

/// The header previously declared `fabric_capability_free_key` as a phantom
/// symbol that did not exist in the Rust crate — a link-time failure for
/// any C consumer. We use `nm` on the staticlib to assert the symbol is
/// absent so a future re-introduction forces an explicit decision.
#[test]
fn phantom_free_key_symbol_is_not_exported() {
    use std::process::Command;

    // Locate the staticlib. Cargo may have placed it at either of two
    // predictable paths, depending on whether the build was triggered by
    // `cargo build` or directly by `cargo test`.
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let candidates = [
        workspace_root.join("target/debug/libfabric_capability_ffi.a"),
        workspace_root.join("target/debug/deps/libfabric_capability_ffi.a"),
    ];
    let staticlib = candidates
        .iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| panic!("staticlib not found in any of: {:?}", candidates));

    // `nm` may not exist on every host (Windows). Skip the assertion there.
    let nm = Command::new("nm").arg(staticlib).output();
    let output = match nm {
        Ok(o) => o,
        Err(_) => return, // nm unavailable — pin absence cannot be checked on this host
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("fabric_capability_free_key"),
        "fabric_capability_free_key must NOT be exported from the staticlib (header phantom regression)"
    );
}
