//! C FFI bindings for `fabric-capability`.
//!
//! This crate exposes a safe `no_std`-compatible C FFI layer using `cbindgen`
//! to generate `fabric_capability.h` from the `fabric_capability_ffi.h` comments.
//!
//! # Safety
//!
//! All functions that accept or return raw pointers are `unsafe`.

use fabric_capability::descriptor::CapabilityDescriptor;
use fabric_capability::{sign, verify, SigningKey};

/// Error codes returned by FFI functions.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FabricError {
    Ok = 0,
    Unsupported = 1,
    Schema = 2,
    Crypto = 3,
    Signature = 4,
    Io = 5,
    Serde = 6,
    NullPointer = 7,
}

impl From<&fabric_capability::Error> for FabricError {
    fn from(e: &fabric_capability::Error) -> Self {
        use fabric_capability::Error::*;
        match e {
            Unsupported(_) => FabricError::Unsupported,
            Schema(_) => FabricError::Schema,
            Crypto(_) => FabricError::Crypto,
            Signature(_) => FabricError::Signature,
            Io(_, _) => FabricError::Io,
            Serde(_) => FabricError::Serde,
            Malformed(_, _) => FabricError::Io,
            BudgetExceeded(_, _, _) => FabricError::Io,
        }
    }
}

// ---------------------------------------------------------------------------
// CapabilityDescriptor
// ---------------------------------------------------------------------------

/// Frees a JSON string allocated by this library.
///
/// # Safety
/// - `ptr` must be a pointer returned by this library.
/// - Must not be called twice on the same pointer.
#[no_mangle]
pub unsafe extern "C" fn fabric_capability_free_string(ptr: *mut std::os::raw::c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = std::ffi::CString::from_raw(ptr);
    }
}

/// Serializes a capability descriptor to a JSON string.
///
/// Caller owns the returned pointer. Free with `fabric_capability_free_string`.
/// Returns `null` on error: invalid UTF-8 in the input, an embedded NUL byte
/// in the output, or a JSON encoding failure.
///
/// # Safety
///
/// - `descriptor_json` must be a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn fabric_capability_to_json(
    descriptor_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    if descriptor_json.is_null() {
        return std::ptr::null_mut();
    }

    // Refuse invalid UTF-8 with a documented NULL — do NOT silently substitute
    // an empty string (the previous behaviour returned `"\"\""` and looked
    // like a successful round-trip).
    let json = match unsafe { std::ffi::CStr::from_ptr(descriptor_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match serde_json::to_string(json) {
        Ok(s) => match std::ffi::CString::new(s) {
            Ok(cstring) => cstring.into_raw(),
            // Embedded NUL byte in the encoded output — return NULL rather
            // than truncate the result.
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns the error message for a `FabricError` code.
///
/// The returned pointer is a static string — do not free it.
#[no_mangle]
pub extern "C" fn fabric_capability_error_message(
    code: FabricError,
) -> *const std::os::raw::c_char {
    let s: &'static str = match code {
        FabricError::Ok => "success",
        FabricError::Unsupported => "unsupported platform",
        FabricError::Schema => "schema validation failed",
        FabricError::Crypto => "cryptographic error",
        FabricError::Signature => "signature verification failed",
        FabricError::Io => "I/O error",
        FabricError::Serde => "serialization error",
        FabricError::NullPointer => "null pointer",
    };
    s.as_ptr() as *const std::os::raw::c_char
}

// ---------------------------------------------------------------------------
// Signing
// ---------------------------------------------------------------------------

/// Signs a capability descriptor JSON string with an Ed25519 key.
///
/// `key_bytes` must be a 32-byte key seed.
/// Returns `FabricError::Ok` on success.
///
/// # Safety
/// - `descriptor_json` must be a valid, null-terminated C string.
/// - `key_bytes` must be a pointer to 32 bytes.
/// - `out_signed` must not be null.
#[no_mangle]
pub unsafe extern "C" fn fabric_capability_sign_json(
    descriptor_json: *const std::os::raw::c_char,
    key_bytes: *const u8,
    out_signed: *mut *mut std::os::raw::c_char,
) -> FabricError {
    if descriptor_json.is_null() || key_bytes.is_null() || out_signed.is_null() {
        return FabricError::NullPointer;
    }

    let json = match unsafe { std::ffi::CStr::from_ptr(descriptor_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return FabricError::Serde,
    };

    let mut descriptor: CapabilityDescriptor = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(_) => return FabricError::Serde,
    };

    // Read exactly 32 bytes from the caller's buffer. The FFI always
    // hands `key_bytes` a 32-byte-aligned pointer (per its contract), so
    // this is just a load, not a length check.
    let key_arr: [u8; 32] = unsafe { *(key_bytes as *const [u8; 32]) };
    let key = SigningKey::from_bytes(&key_arr);

    if let Err(e) = sign(&mut descriptor, &key) {
        return FabricError::from(&e);
    }

    match serde_json::to_string(&descriptor) {
        Ok(s) => match std::ffi::CString::new(s) {
            // Embedded NUL byte in the encoded descriptor — report Serde
            // rather than panicking across the FFI boundary.
            Err(_) => FabricError::Serde,
            Ok(cstring) => {
                unsafe {
                    *out_signed = cstring.into_raw();
                }
                FabricError::Ok
            }
        },
        Err(_) => FabricError::Serde,
    }
}

/// Verifies a signed capability descriptor JSON against a signing key.
///
/// `seed_bytes` must be a 32-byte Ed25519 seed — the same bytes returned by
/// `fabric_capability_generate_key`. The library derives the verification
/// key from the seed internally, so the C consumer does not need to know
/// the distinction between seed and public key.
///
/// Returns `FabricError::Ok` if the signature is valid.
///
/// # Safety
/// - `descriptor_json` must be a valid, null-terminated C string.
/// - `seed_bytes` must be a pointer to 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn fabric_capability_verify_json(
    descriptor_json: *const std::os::raw::c_char,
    seed_bytes: *const u8,
) -> FabricError {
    if descriptor_json.is_null() || seed_bytes.is_null() {
        return FabricError::NullPointer;
    }

    let json = match unsafe { std::ffi::CStr::from_ptr(descriptor_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return FabricError::Serde,
    };

    let descriptor: CapabilityDescriptor = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(_) => return FabricError::Serde,
    };

    // The 32 bytes are a SEED (not a public key). The underlying ed25519-dalek
    // 3.0 `VerifyingKey::from_bytes` will panic on seed-shaped bytes via the
    // upstream `.expect("invalid key")` — an FFI-unsafe panic. We go through
    // `SigningKey::from_bytes` (which never panics) and derive the verification
    // key from it. This keeps the FFI's sign/verify interface symmetric: the
    // same 32 bytes produced by `fabric_capability_generate_key` work for both.
    let seed_arr: [u8; 32] = unsafe { *(seed_bytes as *const [u8; 32]) };
    let sk = SigningKey::from_bytes(&seed_arr);
    let vk = sk.verification_key();

    match verify(&descriptor, &vk) {
        Ok(()) => FabricError::Ok,
        Err(e) => FabricError::from(&e),
    }
}

/// Generates a new random Ed25519 signing key.
///
/// Generates a new signing key and writes the 32-byte seed directly into the
/// caller's buffer. There is no heap allocation — the caller does NOT need to
/// free the returned bytes. The buffer must outlive this call.
///
/// # Safety
///
/// - `out_key` must be a valid pointer to a writable `[u8; 32]` buffer.
#[no_mangle]
pub unsafe extern "C" fn fabric_capability_generate_key(out_key: *mut [u8; 32]) -> FabricError {
    if out_key.is_null() {
        return FabricError::NullPointer;
    }

    let key = SigningKey::generate();
    unsafe {
        *out_key = key.to_bytes();
    }
    FabricError::Ok
}

/// Returns the key fingerprint (key_id) for a given key as a hex string.
///
/// The returned pointer must be freed by the caller with `fabric_capability_free_string`.
///
/// # Safety
///
/// - `key_bytes` must be a valid pointer to 32 bytes of Ed25519 seed.
#[no_mangle]
pub unsafe extern "C" fn fabric_capability_key_id(
    key_bytes: *const u8,
) -> *mut std::os::raw::c_char {
    if key_bytes.is_null() {
        return std::ptr::null_mut();
    }

    // The 32 bytes are a SEED. SigningKey::from_bytes accepts any 32 bytes.
    let key_arr: [u8; 32] = unsafe { *(key_bytes as *const [u8; 32]) };
    let key = SigningKey::from_bytes(&key_arr);
    let key_id = key.key_id();

    std::ffi::CString::new(key_id.to_string())
        .map(|c| c.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
