//! Shared helpers for `fabric-capability-ffi` ABI integration tests.
//!
//! Cargo's integration test layout treats every `tests/*.rs` as a separate
//! test binary, EXCEPT for `tests/common/mod.rs` which is treated as a
//! shared module. By declaring `mod common;` in each test file, all helpers
//! below are reusable across the suite.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// `strlen` is the canonical way to verify a `*const c_char` is a real
// NUL-terminated C string. We declare it locally so the tests do not need
// to depend on the `libc` crate (which is not in this crate's dev-deps).
//
// Not every integration test binary uses this; suppress the resulting
// `dead_code` warning so individual test files can opt in without noise.
#[allow(dead_code)]
extern "C" {
    pub fn strlen(s: *const c_char) -> usize;
}

/// Build a leaked `*const c_char` from a Rust `&str`. The caller is
/// responsible for freeing it (or for handing it to `fabric_capability_*`
/// and observing the documented ownership rules).
#[allow(dead_code)] // not every test binary uses this helper
pub fn cstr(s: &str) -> *const c_char {
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
#[allow(dead_code)] // not every test binary uses this helper
pub unsafe fn cstr_to_string_lossy(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    CStr::from_ptr(p).to_string_lossy().into_owned()
}
