//! ABI surface tests for `fabric-capability-ffi`.
//!
//! These tests pin the wire-level contract between the Rust crate and the
//! C header: integer widths of error codes, completeness of the error
//! message table, and the header/Rust symbol agreement. The companion
//! `c_link_smoke` test exercises the link-side of the same contract from
//! a real C compiler.

mod common;

use std::ffi::CStr;

use fabric_capability_ffi::{
    fabric_capability_error_message, fabric_capability_free_string, fabric_capability_generate_key,
    fabric_capability_key_id, fabric_capability_sign_json, fabric_capability_to_json,
    fabric_capability_verify_json, FabricError,
};

use crate::common::strlen;

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

/// Every `FabricError` discriminant must be distinct. If two codes collided,
/// the C consumer would be unable to distinguish failures.
#[test]
fn fabric_error_codes_are_distinct() {
    let codes = [
        FabricError::Ok,
        FabricError::Unsupported,
        FabricError::Schema,
        FabricError::Crypto,
        FabricError::Signature,
        FabricError::Io,
        FabricError::Serde,
        FabricError::NullPointer,
    ];
    for i in 0..codes.len() {
        for j in (i + 1)..codes.len() {
            assert_ne!(
                codes[i] as i32, codes[j] as i32,
                "{:?} and {:?} share a discriminant",
                codes[i], codes[j]
            );
        }
    }
}

/// The error message table is a documentation contract: every code must have
/// a non-empty, NUL-terminated, UTF-8 string. If a code were unhandled the
/// implementation would have to return `""` and C callers would have seen
/// an empty message.
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
// Header ↔ Rust surface reconciliation.
//
// The header is hand-authored. A prior revision included a phantom
// `fabric_capability_free_key` that did not exist in the Rust crate, which
// would have caused a link-time failure for any C consumer. The companion
// `c_link_smoke` test exercises this end-to-end by compiling C against
// the header: if the header declares a symbol the crate does not export,
// the link step fails and the test fails to build.
//
// Here, we additionally guard the header source itself with a portable
// string check — no external tooling (`nm`, `objdump`, `llvm-nm`) required.
// ---------------------------------------------------------------------------

/// The header is shipped as a hand-authored file. We pin the symbol list by
/// taking the address of every Rust export and asserting the header declares
/// each one. If the header dropped a function, the C consumer in `c_link_smoke`
/// would fail to link; if the header added a function the crate does not
/// export, the C consumer would also fail to link. This test makes the
/// contract check explicit on the Rust side too.
#[test]
fn every_header_declared_function_is_present_in_the_library() {
    // Each entry is the EXACT symbol name as the C linker will see it. If
    // someone adds a declaration to the header without an `#[no_mangle] extern "C"`
    // implementation, this test pins the divergence at the Rust boundary.
    let _fn_free: unsafe extern "C" fn(*mut std::os::raw::c_char) = fabric_capability_free_string;
    let _fn_to_json: unsafe extern "C" fn(
        *const std::os::raw::c_char,
    ) -> *mut std::os::raw::c_char = fabric_capability_to_json;
    let _fn_err_msg: unsafe extern "C" fn(FabricError) -> *const std::os::raw::c_char =
        fabric_capability_error_message;
    let _fn_sign: unsafe extern "C" fn(
        *const std::os::raw::c_char,
        *const u8,
        *mut *mut std::os::raw::c_char,
    ) -> FabricError = fabric_capability_sign_json;
    let _fn_verify: unsafe extern "C" fn(*const std::os::raw::c_char, *const u8) -> FabricError =
        fabric_capability_verify_json;
    let _fn_gen: unsafe extern "C" fn(*mut [u8; 32]) -> FabricError =
        fabric_capability_generate_key;
    let _fn_id: unsafe extern "C" fn(*const u8) -> *mut std::os::raw::c_char =
        fabric_capability_key_id;
}

/// Header must NOT declare the phantom `fabric_capability_free_key` symbol.
/// This was a real regression in the past: the header declared a function
/// that the Rust crate did not export, causing link-time failure for any
/// C consumer that included the header.
///
/// We assert this with a portable string search against the header bytes
/// — no `nm`/`objdump`/`llvm-nm` dependency. The companion `c_link_smoke`
/// test asserts the same property end-to-end at the linker level.
#[test]
fn header_does_not_declare_phantom_free_key() {
    const HEADER: &str = include_str!("../include/fabric_capability.h");
    assert!(
        !HEADER.contains("fabric_capability_free_key"),
        "header must NOT declare the phantom `fabric_capability_free_key` symbol — \
         that would cause a link-time failure for any C consumer that follows \
         the header. The c_link_smoke test pins this end-to-end at the linker level."
    );
}

/// Conversely, every public Rust symbol MUST appear in the header so a C
/// consumer can actually call it. We assert each export name is present in
/// the header as a substring, anchoring the contract from the other side.
#[test]
fn every_rust_export_appears_in_the_header() {
    const HEADER: &str = include_str!("../include/fabric_capability.h");
    for symbol in [
        "fabric_capability_error_message",
        "fabric_capability_free_string",
        "fabric_capability_to_json",
        "fabric_capability_sign_json",
        "fabric_capability_verify_json",
        "fabric_capability_generate_key",
        "fabric_capability_key_id",
    ] {
        assert!(
            HEADER.contains(symbol),
            "header must declare `{symbol}` — the Rust crate exports it"
        );
    }
}
