# 0029 — Rust vs Go Port Policy

## Status

Accepted — 2026-09-05.

## Date

2026-09-05.

## Deciders

phenotype-fabric working group.

## Traceability

- PF-WP-011 (capability-probe vs NVMS-manifest cross-checker) — spec 018 / ADR-0027
- PF-WP-011 delivered as Go-native in `cmd/checker/` (9 sub-tests passing)
- Rust port stalled in `crates/fabric-checker/` (workspace-excluded, source preserved)

## Context

Across the 2026-09-01 → 2026-09-05 session chain, three Rust crates
(`fabric-workspace`, `fabric-cli`, `fabric-checker`) repeatedly hit cascading
type-mismatch loops: 50+ errors per rewrite, multiple rewrite attempts each.
The root cause is consistent and documented in WORKLOG.md: writing against
**invented** type shapes instead of reading the authoritative Rust source
first.

By contrast, the same capability-checker logic was delivered in Go
(`cmd/checker/`) in a single focused pass — 657 lines stdlib-only, 9 tests
passing — by reading `crates/fabric-capability/src/descriptor.rs` and
`crates/phenotype-nvms-adapter/src/required.rs` **before** writing the
Go mirror types.

The question is now: **should we keep two implementations (Rust port +
Go checker) or pick one and drop the other?**

## Decision

The Go implementation (`cmd/checker/`) is the **canonical reference** for
PF-WP-011 in R0. The Rust port (`crates/fabric-checker/`) is **deferred to R1**
(when Rust-side distribution becomes a real requirement) and **only if**
the Go implementation has demonstrated 6+ months of stability in production.

The decision criteria for promoting the Rust port back to active status:

1. **A Rust consumer needs to call the checker directly** (e.g. the
   `fabric-cli` Rust crate, or a Rust-based sharecli integration). In that
   case, an FFI shim to the Go binary is the first answer; a Rust port is
   the second.
2. **DataPipe's R2 trust-root model** (ADR-0017) requires signed
   `CapabilityDescriptor`s to be verified inside a Rust runtime. The checker
   becomes part of that signature pipeline.
3. **The Go checker has had no semantic drift for 6 months** in production.
   Drift is measured against the fixtures in `cmd/checker/testdata/`
   verified via the pattern in ADR-0028.

Until two of those three are true, the Rust port remains **frozen at the
R0 spec** (source preserved untracked, no commits, no CI). It is not a
maintenance burden.

## Why not keep both at parity forever

Three reasons:

1. **Drift cost**: every change to `Capabilities` / `RequiredCapabilities` /
   `Manifest` must be made in two languages. The Go checker caught a real
   field-name drift in 2026-09-05 that the Rust port would have missed
   for weeks.

2. **Test budget**: the Go checker has 9 tests + 12 fixture-driven E2E
   pairs. The Rust port would need an equivalent Rust integration test
   suite (we have the `fixtures_roundtrip.rs` seed). That doubles the
   reviewer burden on every spec change.

3. **Distribution model**: the deliverable is `fabric cap` / `fabric
   checker` CLI. The Go binary is a static cross-platform executable that
   doesn't need Rust toolchain. Anyone running fabric from a non-Rust
   host already uses Go.

## Why not just Go-only

The Rust port exists for a reason — when Rust is the right runtime
(e.g. embedded agent, in-tree fabric integration, R2 trust-root chain).
Keeping the source preserved means we can re-activate it when those
scenarios materialize, without re-deriving the checker logic from scratch.

## Decision matrix for future port requests

| Request | Decision | Rationale |
|:--|:--|:--|
| "Add Rust port because someone wants to call it from Rust" | **Implement an FFI shim** to the Go binary first | Cheaper, avoids API drift |
| "Add Rust port because Rust toolchain is mandated by the host" | **Evaluate; if host has Rust, FFI shim is still preferred** | Reduces surface area |
| "Add Rust port because we need signed-descriptor verification inside a Rust trust-root chain" | **Reactivate the Rust port per this ADR** | Real demand |
| "Add Rust port because the Go checker has a bug" | **Fix the Go checker; do not mirror the bug in Rust** | Single source of truth |

## Consequences

- **Positive**: the Go checker is the single source of truth for PF-WP-011.
  All changes go through one implementation, one test suite, one verifier.
- **Positive**: the Rust port is preserved at R0 spec, so re-activation is
  cheap when needed.
- **Cost**: if a Rust consumer appears before R2, we pay for an FFI
  shim layer (a few hundred lines).
- **Risk**: the Rust port ages out. When R2 needs it, the port will need
  a refresh against the then-current Rust types. Mitigated by the
  testdata verification pattern in ADR-0028 (the Rust port's `examples/verify_fixtures.rs`
  is the first thing that catches type drift).

## References

- `cmd/checker/` — the canonical Go implementation (657 LoC, 9 tests)
- `crates/fabric-checker/` — the deferred Rust port (source preserved)
- `cmd/checker/testdata/` — the 12 fixtures verified by ADR-0028
- `adr/0027-checker-decision-taxonomy.md` — the reason-code taxonomy
- `adr/0028-testdata-verification-pattern.md` — the verification pattern
- WORKLOG.md — "Wrote Go checker because Rust drift loop failed 3 times"
