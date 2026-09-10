# Spec 026 Tasks — trust operator CLI

## Phase 0 (done in prior turn)
- [x] Read `crates/fabric-capability/src/trust_root.rs` end-to-end
- [x] Read `crates/fabric-capability/src/signing.rs` end-to-end
- [x] Read `crates/fabric-capability/src/descriptor.rs` end-to-end
- [x] Read `cmd/wire/{wire,wire_test}.go` for operator-committed patterns

## Phase 1 — implementation
- [ ] S026-01: `cmd/trust/go.mod` (module declaration, no deps)
- [ ] S026-02: `cmd/trust/trust.go` (5 subcommands: init/issue/sign/revoke/inspect)
- [ ] S026-03: `cmd/trust/testdata/{root,intermediate,revocation}.json` (3 fixture files)
- [ ] S026-04: `cmd/trust/trust_test.go` (≥12 tests covering each subcommand + JSON envelope round-trip + Rust wire compat)

## Phase 2 — verification
- [ ] S026-05: `go build ./... cmd/trust/` (binary builds)
- [ ] S026-06: `go test ./... cmd/trust/` (all tests pass)
- [ ] S026-07: `cargo test --workspace` (no regressions)
- [ ] S026-08: 4/4 spec checks (`check_manifest.py` + 3 others)

## Phase 3 — commit + push
- [ ] S026-09: Update `specs/INDEX.md` (add spec 026 row)
- [ ] S026-10: Regen `MANIFEST.sha256` for the 4 new files
- [ ] S026-11: WORKLOG entry for spec 026
- [ ] S026-12: `git commit -m "feat(trust): operator CLI for trust-root issuance (PF-WP-018, spec 026)"`
- [ ] S026-13: `git push origin main` (verify on PhenoFabric)
- [ ] S026-14: Update `meta/PHENOTYPE_ARCHITECTURE.md`
- [ ] S026-15: Commit + push in `meta/` repo
