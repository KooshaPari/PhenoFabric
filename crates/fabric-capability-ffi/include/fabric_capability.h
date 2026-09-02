/*
 * fabric-capability-ffi — C ABI for fabric-capability
 *
 * Hand-authored header. Mirrors the Rust surface in `src/lib.rs`.
 * Tested against the Rust `fabric-capability-ffi` crate at v0.1.0.
 *
 * Conventions:
 *  - All pointer parameters must be non-null unless documented otherwise.
 *  - Returned strings are UTF-8, NUL-terminated, and must be released with
 *    `fabric_capability_free_string` (or `fabric_capability_free_key` for
 *    raw byte buffers).
 *  - Error codes are `FabricError` (see enum below).
 *
 * Thread safety: all functions are safe to call from any thread.
 */

#ifndef FABRIC_CAPABILITY_FFI_H
#define FABRIC_CAPABILITY_FFI_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* --- Error codes -------------------------------------------------------- */

typedef enum {
    FABRIC_OK              = 0,
    FABRIC_UNSUPPORTED     = 1,
    FABRIC_SCHEMA          = 2,
    FABRIC_CRYPTO          = 3,
    FABRIC_SIGNATURE       = 4,
    FABRIC_IO              = 5,
    FABRIC_SERDE           = 6,
    FABRIC_NULL_POINTER    = 7,
} FabricError;

/* Returns a static, NUL-terminated error message for `code`. Do NOT free. */
const char *fabric_capability_error_message(FabricError code);

/* --- Memory ------------------------------------------------------------- */

/* Free a string returned by this library. Safe to call with NULL. */
void fabric_capability_free_string(char *s);

/* Free a raw key buffer returned by this library. Safe to call with NULL. */
void fabric_capability_free_key(unsigned char *p);

/* --- Capability descriptor JSON round-trip ----------------------------- */

/* Re-serialize a JSON descriptor (validates JSON, canonicalizes ordering).
 * Returns a freshly-allocated JSON string, or NULL on error. */
char *fabric_capability_to_json(const char *descriptor_json);

/* --- Signing ------------------------------------------------------------ */

/* Sign a capability descriptor JSON.
 * `descriptor_json` must be a valid, null-terminated C string.
 * `key_bytes` must point to 32 bytes of Ed25519 seed.
 * On success, `*out_signed` is set to a freshly-allocated signed JSON string
 * (free with `fabric_capability_free_string`).
 * Returns FABRIC_OK on success. */
FabricError fabric_capability_sign_json(const char *descriptor_json,
                                       const unsigned char *key_bytes,
                                       char **out_signed);

/* Verify a signed capability descriptor against a verification key.
 * `descriptor_json` must be a valid, null-terminated C string.
 * `vk_bytes` must point to 32 bytes of Ed25519 public key.
 * Returns FABRIC_OK if the signature is valid, otherwise an error code. */
FabricError fabric_capability_verify_json(const char *descriptor_json,
                                          const unsigned char *vk_bytes);

/* --- Key management ----------------------------------------------------- */

/* Generate a new random Ed25519 signing key (32 bytes of seed).
 * On success, `out_key[32]` is filled with the key seed.
 * Free raw key memory with `fabric_capability_free_key`. */
FabricError fabric_capability_generate_key(unsigned char out_key[32]);

/* Returns the key fingerprint (blake3 hex) for a given key.
 * Caller frees the returned string with `fabric_capability_free_string`. */
char *fabric_capability_key_id(const unsigned char *key_bytes);

#ifdef __cplusplus
}
#endif

#endif /* FABRIC_CAPABILITY_FFI_H */
