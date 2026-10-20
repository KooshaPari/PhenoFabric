//! The C source that the `c_link_smoke` test compiles and links against the
//! `fabric-capability-ffi` staticlib.
//!
//! The program is intentionally small. It exercises every FFI function the
//! library exports and exits 0 only if each call returns the expected
//! outcome. If the C ↔ Rust contract breaks (ABI drift, header missing
//! symbol, wrong enum value, etc.) the C program will fail to compile, fail
//! to link, or return non-zero at runtime.
//!
//! IMPORTANT: this program IS the test. If it changes, the assertions it
//! pins change. Every branch that does NOT exit with EXIT_FAILURE represents
//! a passed contract assertion.

/// The C program source, verbatim.
pub const C_PROGRAM: &str = r#"
// fabric-capability-ffi cross-language smoke test.
// Every branch that does NOT exit with EXIT_FAILURE represents a passed
// contract assertion.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fabric_capability.h"

#define PASS(name) do { printf("  ok %s\n", name); fflush(stdout); } while (0)
#define FAIL(name, why) do { fprintf(stderr, "FAIL %s: %s\n", name, why); fflush(stderr); exit(EXIT_FAILURE); } while (0)

int main(void) {
    // --- error_message: every code must return a non-NULL non-empty UTF-8-ish string.
    for (FabricError c = 0; c <= FABRIC_NULL_POINTER; c++) {
        const char *m = fabric_capability_error_message(c);
        if (!m) FAIL("error_message NULL", "returned NULL");
        if (strlen(m) == 0) FAIL("error_message empty", "returned empty string");
    }
    PASS("error_message");

    // --- free_string(NULL): must be a safe no-op.
    fabric_capability_free_string(NULL);
    PASS("free_string(NULL)");

    // --- to_json(NULL): must return NULL.
    if (fabric_capability_to_json(NULL) != NULL) FAIL("to_json(NULL)", "did not return NULL");
    PASS("to_json(NULL)");

    // --- generate_key: must populate 32 bytes; NULL out returns FABRIC_NULL_POINTER.
    unsigned char seed[32];
    if (fabric_capability_generate_key(NULL) != FABRIC_NULL_POINTER)
        FAIL("generate_key(NULL)", "did not return FABRIC_NULL_POINTER");
    if (fabric_capability_generate_key(seed) != FABRIC_OK)
        FAIL("generate_key(ok)", "did not return FABRIC_OK");
    PASS("generate_key");

    // --- key_id: must return a NUL-terminated heap string (must NULL-safe).
    if (fabric_capability_key_id(NULL) != NULL) FAIL("key_id(NULL)", "did not return NULL");
    char *kid = fabric_capability_key_id(seed);
    if (!kid) FAIL("key_id(ok)", "returned NULL");
    if (strlen(kid) == 0) FAIL("key_id(ok)", "returned empty string");
    // Determinism: same seed -> same fingerprint.
    char *kid2 = fabric_capability_key_id(seed);
    if (!kid2 || strcmp(kid, kid2) != 0)
        FAIL("key_id deterministic", "two calls produced different fingerprints");
    fabric_capability_free_string(kid);
    fabric_capability_free_string(kid2);
    PASS("key_id");

    // --- sign + verify round-trip with the generated seed.
    // The FFI's sign and verify both take a 32-byte SEED. Verify derives the
    // verification key from the seed internally, so the same 32 bytes used
    // to sign must also verify the signed descriptor.
    const char *desc =
        "{\"node_id\":\"00000000-0000-7000-8000-000000000001\","
        "\"epoch\":1,"
        "\"schema_version\":\"phenotype.fabric.capability_descriptor/1\","
        "\"probed_at\":\"2026-01-01T00:00:00Z\","
        "\"probe_version\":\"0.1.0\","
        "\"topology_hash\":\"deadbeef\","
        "\"capabilities\":{},"
        "\"signatures\":[]}";
    char *signed_desc = NULL;
    FabricError e = fabric_capability_sign_json(desc, seed, &signed_desc);
    if (e != FABRIC_OK) FAIL("sign_json(ok)", "did not return FABRIC_OK");
    if (!signed_desc) FAIL("sign_json(ok)", "out_signed was NULL");
    if (!strstr(signed_desc, "\"signatures\""))
        FAIL("sign_json(ok)", "signed descriptor lacks signatures field");
    PASS("sign_json");

    // Same-seed verify must succeed: the descriptor was signed with this
    // exact seed, and the FFI's verify derives the matching verification key.
    FabricError vself = fabric_capability_verify_json(signed_desc, seed);
    if (vself != FABRIC_OK)
        FAIL("verify_json(same seed)", "verify under the same seed that signed did not return FABRIC_OK");
    PASS("verify_json(same seed)");

    // --- NULL handling for sign_json.
    char *placeholder = NULL;
    if (fabric_capability_sign_json(NULL, seed, &placeholder) != FABRIC_NULL_POINTER)
        FAIL("sign_json(NULL desc)", "did not return FABRIC_NULL_POINTER");
    if (fabric_capability_sign_json(desc, NULL, &placeholder) != FABRIC_NULL_POINTER)
        FAIL("sign_json(NULL key)", "did not return FABRIC_NULL_POINTER");
    if (fabric_capability_sign_json(desc, seed, NULL) != FABRIC_NULL_POINTER)
        FAIL("sign_json(NULL out)", "did not return FABRIC_NULL_POINTER");
    PASS("sign_json(NULL *)");

    // --- verify_json NULL handling.
    if (fabric_capability_verify_json(NULL, seed) != FABRIC_NULL_POINTER)
        FAIL("verify_json(NULL desc)", "did not return FABRIC_NULL_POINTER");
    if (fabric_capability_verify_json(signed_desc, NULL) != FABRIC_NULL_POINTER)
        FAIL("verify_json(NULL seed)", "did not return FABRIC_NULL_POINTER");
    PASS("verify_json(NULL *)");

    // --- Cross-seed rejection: a different 32-byte seed must not verify.
    unsigned char other_seed[32];
    memset(other_seed, 0xCC, sizeof(other_seed));
    FabricError verr = fabric_capability_verify_json(signed_desc, other_seed);
    if (verr == FABRIC_OK)
        FAIL("verify_json(other seed)", "forged signature verified under a different seed");
    PASS("verify_json(rejects other seed)");

    // --- The FFI's contract for the key is "a pointer to 32 bytes" — the
    // length is fixed by the signature, not by the input. There is no
    // `bad key len` runtime error to surface here: the consumer must
    // guarantee the buffer size. We instead verify that the key bytes
    // are actually USED (round-trip via sign+verify below).

    // --- Serde error on malformed descriptor.
    char *out_buf = NULL;
    if (fabric_capability_sign_json("not json {", seed, &out_buf) != FABRIC_SERDE)
        FAIL("sign_json(bad desc)", "did not return FABRIC_SERDE");
    PASS("sign_json(Serde on bad desc)");

    // --- Header must NOT export `fabric_capability_free_key` (phantom fix).
    // The C compile would NOT directly catch a phantom declaration, but
    // if the header declared a phantom AND the program called it, the link
    // step would fail. We rely on tests/abi.rs for the portable absence
    // assertion (header string search). Here we pin the positive half:
    // every symbol we use below MUST resolve at link time.

    fabric_capability_free_string(signed_desc);
    printf("c_link_smoke: all FFI contracts verified from C\n");
    return 0;
}
"#;
