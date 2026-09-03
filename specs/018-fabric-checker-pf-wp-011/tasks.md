# Tasks — Spec 018 (fabric-checker, PF-WP-011)

## Phase 0 — Spec & ADR (this turn)

- [ ] **T018-01** Author spec 018 (meta, spec, plan, tasks).
- [ ] **T018-02** Author ADR-0027 (`adr/0027-checker-decision-taxonomy.md`)
  with the full ReasonCode list and severity assignments.
- [ ] **T018-03** Create the `fabric-checker` crate skeleton: `Cargo.toml`
  with `fabric-capability` + `phenotype-nvms-adapter` + `serde` +
  `serde_json` + `thiserror` deps; `src/lib.rs` with public surface stub.
- [ ] **T018-04** Add `crates/fabric-checker` to the workspace members.
- [ ] **T018-05** Author `verification/checker-fixtures/manifest-minimal.yaml`
  and `verification/checker-fixtures/capability-minimal.json` (the
  matching baseline pair).
- [ ] **T018-06** Author the test matrix fixtures:
  - `manifest-mem-short.yaml` (rejects on memory)
  - `manifest-cores-short.yaml` (rejects on cores)
  - `manifest-rt-required.yaml` (rejects on missing RT)
  - `manifest-trust-higher.yaml` (rejects on trust)
  - `manifest-compat-warn.yaml` (admits with notes on compat)
  - `manifest-gpu-mismatch.yaml` (admits with notes on GPU count)
  - `manifest-accel-tie.yaml` (admits with notes on accelerator)
  - `capability-fingerprint-mismatch.json` (rejects on fingerprint)
- [ ] **T018-07** Update `specs/INDEX.md` with row 018 and dependency spine.

## Phase 1 — Rust implementation (R1.0)

- [ ] **T018-08** Implement `Decision` enum (Admit / AdmitWithNotes / Reject).
- [ ] **T018-09** Implement `ReasonCode` enum (the 20+ variants from ADR-0027).
- [ ] **T018-10** Implement `Severity` enum (Hard / Soft).
- [ ] **T018-11** Implement `CheckOutcome` struct (reasons + per-reason
  descriptor-side and manifest-side numbers).
- [ ] **T018-12** Implement `Checker` type with the 20+ check functions,
  one per ReasonCode variant, each pure `(&Descriptor, &Manifest) -> Vec<ReasonCode>`.
- [ ] **T018-13** Implement `Checker::check()` that composes all checks
  and folds their results into a single `Decision`.
- [ ] **T018-14** Add unit tests: at least one test per ReasonCode
  variant (admit / admit-with-notes / reject matrix).
- [ ] **T018-15** Add integration tests against the fixtures in
  `verification/checker-fixtures/`.
- [ ] **T018-16** Wire into `fabric-cli` as `fabric check` subcommand
  (after CLI rewrite is unblocked).

## Phase 2 — Go reference (R1.0)

- [ ] **T018-17** Implement `cmd/checker/main.go` with bit-identical
  decision semantics on the canonical fixtures.
- [ ] **T018-18** Add the Go checker to the spec-validation CI workflow.

## Phase 3 — Workspace integration (R1.5)

- [ ] **T018-19** Wire `fabric-workspace` (specs/017) to call
  `Checker::check()` at workspace creation: Admit / AdmitWithNotes
  proceed, Reject blocks.
- [ ] **T018-20** Surface the reasons in the workspace state file
  (per ADR-0026 `state-promoted` event with the full `CheckOutcome`).

## Definition of done for the spec

- All tasks above checked.
- `cargo test -p fabric-checker` shows ≥ 25 passing tests.
- `go test ./cmd/checker/...` shows ≥ 20 passing tests.
- ADR-0027 stable (Accepted) with the full ReasonCode list.
- Spec 018 referenced from `specs/INDEX.md` and from PHENOTYPE_ARCHITECTURE.md.
