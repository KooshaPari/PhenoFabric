# Plan — Spec 026 implementation

## Phase 0 (read authoritative sources first — per ADR-0028)

Done:
- `crates/fabric-capability/src/trust_root.rs` — full read (Authority, RevocationEntry, RevocationList, TrustStore, ChainVerification, RevocationReason, TrustError, AuthorityError, MAX_CHAIN_DEPTH=2)
- `crates/fabric-capability/src/signing.rs` — full read (SigningKey, VerificationKey, sign_bytes/verify_bytes free fns)
- `crates/fabric-capability/src/descriptor.rs` — full read (CapabilityDescriptor::canonical_bytes() rule: strip signature before hashing)
- `cmd/wire/wire.go` + `cmd/wire/wire_test.go` — operator-committed patterns (no Rust consumption, stdlib only, subcommands in a switch)

Key findings:
- `Authority::canonical_bytes()` strips the `signature` field before SHA-256, matches spec 021 §3 step 2
- `Authority::signed_by` takes `child_key + parent_signing_key + parent + name + not_after` — parent signs the child
- `MAX_CHAIN_DEPTH = 2`
- `RevocationList` is signed by the **root** (not the leaf's parent)
- `key_id` = first 8 hex chars of SHA-256(verification_key_bytes) — same algorithm used in `Authority::key_id()` getter

## Phase 1 (impl)

1. `cmd/trust/trust.go` (~280 LoC): 5 subcommands, JSON marshal/unmarshal matching spec 021 wire format
2. `cmd/trust/trust_test.go` (~12 tests): each subcommand + JSON envelope round-trip + wire compat with Rust
3. `cmd/trust/go.mod` (module declaration, no deps)
4. `cmd/trust/testdata/` (3 fixture files: root authority, intermediate, revocation list)

## Phase 2 (verification)

1. `go test ./... cmd/trust/` — all 12+ tests pass
2. `go build ./... cmd/trust/` — binary builds
3. `cargo test --workspace` — no regressions
4. `python3 program/scripts/check_manifest.py` — manifest includes 4 new files
5. `python3 program/scripts/check_json_schemas.py` + `check_openapi.py` + `check_links.py` — 4/4 spec checks

## Phase 3 (commit + push)

1. `git add specs/026-trust-operator-cli/ cmd/trust/ MANIFEST.sha256 specs/INDEX.md WORKLOG.md`
2. `git commit -m "feat(trust): operator CLI for trust-root issuance (PF-WP-018, spec 026)"`
3. `git push origin main`
4. Update `meta/PHENOTYPE_ARCHITECTURE.md` with the R2 wedge #5 entry
5. Commit + push in `meta/` too

## Risks

- **Wire format drift**: spec 021 §3 must match Rust `serde` derive order. Mitigated by Phase 0 read of `trust_root.rs` + the parallel Rust `wire_compat.rs` test that consumes `cmd/trust` output.
- **Signature scheme mismatch**: must use ed25519 with 64-byte signatures, base64-encoded. Mitigated by reading `signing.rs` in Phase 0.
- **Chain depth enforcement**: MAX_CHAIN_DEPTH=2 must match Rust. Mitigated by Phase 0 read.
