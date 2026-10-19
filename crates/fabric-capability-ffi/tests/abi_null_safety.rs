//! NULL-pointer and memory-ownership tests for `fabric-capability-ffi`.
//!
//! Every function in the FFI that accepts a raw pointer must reject NULL
//! explicitly — a NULL deref across the FFI boundary will abort the C
//! process. These tests take every pointer parameter, in turn, to NULL
//! while keeping the others valid, and assert the documented
//! `FabricError::NullPointer` or `NULL` return. They also pin the
//! ownership/free semantics of the strings returned by the library.

mod common;

use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

use fabric_capability_ffi::{
    fabric_capability_free_string, fabric_capability_generate_key, fabric_capability_key_id,
    fabric_capability_sign_json, fabric_capability_to_json, fabric_capability_verify_json,
    FabricError,
};

use crate::common::{cstr, cstr_to_string_lossy};

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
    let a = std::ffi::CString::new("alpha").unwrap().into_raw();
    let b = std::ffi::CString::new("bravo").unwrap().into_raw();

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
            fabric_capability_sign_json(ptr::null(), key.as_ptr(), (&mut out) as *mut _),
            FabricError::NullPointer
        );
        assert_eq!(
            fabric_capability_sign_json(json, ptr::null(), (&mut out) as *mut _),
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
// UTF-8 handling — distinguishes a successful round-trip from a swallowed
// error. The previous implementation used `unwrap_or_default()` on a UTF-8
// decode failure, returning the JSON encoding of an empty string; the
// caller could not tell their descriptor from `""`. The current contract
// returns NULL.
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
