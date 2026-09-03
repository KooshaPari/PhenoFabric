# Plan — Spec 018 (fabric-checker, PF-WP-011)

## Phase 0 (this spec, R0.5)

- Author spec 018 (this document).
- Capture decision taxonomy in ADR-0027.
- Stand up the crate skeleton (`fabric-checker`); it does *not* need to
  compile yet — just declare its public surface in `lib.rs` and Cargo.toml.
- Ship the canonical JSON fixtures for the test matrix in
  `verification/checker-fixtures/`.

## Phase 1 (R1.0)

- Implement `Decision` / `ReasonCode` / `Severity` / `CheckOutcome` enums
  and structs (Rust).
- Implement the hard-coded check functions, one per ReasonCode variant:
  - `check_memory`, `check_cpu_cores`, `check_gpu_memory`, etc.
- Each check is a pure function: `(&Descriptor, &Manifest) -> Vec<ReasonCode>`.
- The `Checker` type wires them together with a default policy.
- Comprehensive unit tests: at least one test per `ReasonCode` variant.
- Wire into `fabric-cli` as the `fabric check` subcommand.

## Phase 2 (R1.0)

- Implement the Go reference in `cmd/checker/main.go` with bit-identical
  decision semantics on the canonical fixtures.
- Add the checker binary to the spec-validation CI workflow.

## Phase 3 (R1.5)

- Integration with `fabric-workspace` (specs/017): workspace creation calls
  the checker; `Admit` and `AdmitWithNotes` proceed, `Reject` blocks.

## Phase 4 (R2)

- Severity overrides per (reason, deployment) class (e.g. CI is Hard,
  dev is Soft).
- Custom policy plugins via WASM (out of scope for R1).

## WP DAG (relevant subset)

```
PF-WP-010 (capability inventory)  ─┐
                                   ├─> PF-WP-011 (checker)  ─> PF-WP-020 (route compiler)
PF-WP-016 (NVMS adapter)          ─┘                              │
                                                                       v
                                                              PF-WP-017 (workspace persistence)
```

## Spec mutation rules

- Adding a new `ReasonCode` variant requires (a) updating this spec, (b)
  updating ADR-0027, (c) adding a fixture, (d) adding a Rust unit test,
  (e) updating the Go reference. No silent additions.
- Changing the *severity* of a reason code (Hard ↔ Soft) is a breaking
  change for callers that branch on the decision; bump the spec version.

## Acceptance for the spec

- All 20+ reason codes listed with a normative definition.
- Decision semantics unambiguous: Hard → Reject, otherwise Admit unless
  Soft → AdmitWithNotes, no Soft → Admit.
- Idempotence + determinism stated explicitly.
- Out-of-scope section prevents scope creep into runtime / migration /
  identity / observability.
