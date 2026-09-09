# Plan 021 — Trust-Root Implementation

**Status:** draft
**Spec:** [spec.md](./spec.md)
**Release gate:** R1
**Date:** 2026-09-08

## Phase 0 — Read source first (ADR-0028 mandatory)

Before writing ANY line in `trust_root.rs`, read in full:

1. `crates/fabric-capability/src/signing.rs` (170 LoC) — `SigningKey`,
   `VerificationKey`, free fns `sign()` / `verify()`. Note the
   `key_id` is base64(public_key_bytes). Note `verify()` loops over
   `descriptor.signatures` looking for one matching the trusted
   `key_id`, then verifies each signature against the inner key.
2. `crates/fabric-capability/src/descriptor.rs` (339 LoC) — note
   `Signature { key_id, alg, sig, signed_at }`, the `canonical_bytes()`
   method strips the `signatures` field before serializing.
3. `crates/fabric-capability/src/error.rs` (52 LoC) — `Error` enum.
   We'll add a `From<Error> for TrustError` impl.
4. `crates/fabric-capability/Cargo.toml` (42 LoC) — confirms `chrono`,
   `ed25519-dalek`, `base64`, `serde`, `thiserror` are all available.
   No new deps needed.

## Phase 1 — Implement types

Order:

1. Constants: `MAX_CHAIN_DEPTH: usize = 2`.
2. `RevocationReason` enum (4 variants).
3. `RevocationEntry` struct.
4. `RevocationList` struct — note the `signature` field MUST come from
   the TrustRoot.
5. `Authority` struct — fields per spec §3.
6. `ChainVerification` struct.
7. `TrustError` enum (thiserror).
8. `From<Error> for TrustError`.
9. `impl Authority { fn canonical_bytes(&self) -> Result<Vec<u8>, Error> }`
   — strips `signature` and emits compact JSON. Same pattern as
   `CapabilityDescriptor::canonical_bytes()`.
10. `TrustStore` struct + `impl TrustStore` with `new`, `add_authority`,
    `set_revocation_list`, `root_key_id`, `verify_chain`.

## Phase 2 — Wire into `lib.rs`

```rust
pub mod trust_root;
// add to re-exports:
pub use trust_root::{Authority, ChainVerification, RevocationEntry, RevocationList,
                     RevocationReason, TrustError, TrustStore};
```

## Phase 3 — Tests

5 unit tests in `trust_root.rs` (cfg test module). 3 integration
tests in `tests/trust_root_chain.rs`.

## Phase 4 — Verification

```bash
cargo test -p fabric-capability                          # all green
cargo test --workspace                                   # all green
python3 program/scripts/check_manifest.py                # 4/4 ✓
python3 program/scripts/check_json_schemas.py            # 4/4 ✓
python3 program/scripts/check_openapi.py                 # 4/4 ✓
python3 program/scripts/check_links.py                   # 4/4 ✓
```

## Phase 5 — Ship

- `git add` the new files + `lib.rs` + `MANIFEST.sha256` regen.
- Commit message references spec 021 + ADR-0031.
- WORKLOG + meta addendum.

## Risks

- **Phase 0 skip = 50-error cascade** (the documented failure mode).
  Mitigation: do not open `trust_root.rs` until all 4 source files are
  read end-to-end.
- **Cargo.toml edit not needed** — all required deps already present.
- **No new spec-check script needed** — manifest + schema checks
  already cover the new files.
