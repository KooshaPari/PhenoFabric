# Tasks 022 — Multi-Tenant Fairness

## S022-01 — Spec 022 documents authored (DONE)
- meta.json + spec.md + plan.md + tasks.md (this file)
- INDEX.md row added

## S022-02 — `leases_fairness.rs` scaffolded
- TenantId, FairnessPolicy, FairnessDecision, DenyReason,
  TenantAccounting, FairnessSnapshot types
- cargo check passes (no impl yet)

## S022-03 — FairnessQueue impl (Fifo)
- `new(Fifo)`
- `try_acquire` round-robin via VecDeque
- `release` updates total_released

## S022-04 — FairShare policy impl
- deficit-based selection
- ties broken by FIFO order
- 1 unit test

## S022-05 — PriorityWeighted impl
- priority group FIFO
- deny LowerPriority if higher-priority tenant waiting
- 1 unit test

## S022-06 — WeightedRoundRobin impl
- per-tenant weight slots
- rotation refill
- 1 unit test

## S022-07 — snapshot + serde round-trip
- FairnessSnapshot Debug + Clone + PartialEq + Eq + Serialize + Deserialize
- 1 unit test for JSON round-trip

## S022-08 — release updates accounting
- snapshot reflects release
- 1 unit test

## S022-09 — pardon() in leases.rs
- Q4-C operator escape hatch
- valid token: "ops:phenotype:default"
- 2 unit tests

## S022-10 — Integration tests
- 5 tests in tests/lease_fairness_integration.rs

## S022-11 — lib.rs re-exports + module registration
- pub mod leases_fairness;
- pub use for the 4 main types

## S022-12 — Verification
- cargo test -p fabric-graph --lib leases_fairness:: → 8/8
- cargo test -p fabric-graph --test lease_fairness_integration → 5/5
- cargo test --workspace → no regressions
- 4/4 spec checks green
- MANIFEST regen

## S022-13 — Commit + WORKLOG + meta addenda
- feat(leases) commit
- WORKLOG entry
- meta/PHENOTYPE_ARCHITECTURE.md addendum

## Cross-deps

- Spec 022 enables PF-WP-050/060 (R2 audio/video surface runtime)
- Spec 022 enables specs/017 (workspace persistence — fairness audit log)
- spec 022 depends on spec 020 (lease integration) — already shipped
- spec 022 depends on spec 019 (surface plane) — already shipped

## Definition of done

- All 13 tasks marked DONE
- All 13 tests green
- 4/4 spec checks green
- Commit + WORKLOG + meta addenda all landed
- HEAD on phenotype-fabric moved forward
- HEAD on meta moved forward
- R1 cockpit: 100%
