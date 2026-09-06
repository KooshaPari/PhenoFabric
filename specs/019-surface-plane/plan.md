# Plan — Spec 019 Surface Plane (PF-WP-015)

## Phase 0 — Read sources (mandatory; see ADR-0028 + WORKLOG rule)

- `crates/fabric-graph/src/lib.rs` — confirm module-level re-exports
- `crates/fabric-graph/src/model.rs` — confirm `NodeId`, `RouteStep`, `ScoreBreakdown`,
  `TopologyEpoch`, `LocalityTier`, `TrustLevel` field shapes (lines 80–740)
- `crates/fabric-graph/src/compile.rs` — confirm `compile(&Topology, &Intent) -> Result<RoutePlan>`
- `crates/fabric-graph/src/planner.rs` — confirm `plan_sequence(&Topology, &str, &[Intent]) -> Result<MultiRoutePlan>`
- `crates/fabric-capability/src/locality.rs` — confirm `LocalityTier` short_code / as_f64

Do not begin Phase 1 until all five sources have been read in the same
session. This is the rule that broke the prior 50-error loops.

## Phase 1 — Type + decision scaffolding (`crates/fabric-graph/src/surface.rs`)

P0: `SurfaceKind` (8 variants)
P0: `SurfaceId` (`String` newtype)
P0: `SurfaceSpec` (5 fields, derives `Serialize/Deserialize/Clone/PartialEq/Debug`)
P0: `LeaseState` (4 variants)
P0: `SurfaceLease` (6 fields, derives)
P0: `RouteBinding` (3 fields, derives)
P0: `admit(&SurfaceSpec, &RoutePlan) -> Decision` (3 reduce cases per acceptance
   criteria; returns existing PF-WP-011 Decision shape)
P0: `fabric-graph/src/decision.rs` (new module — `Decision`, `Severity`, `ReasonCode`
   enums with `String()`; ReasonCode variants:
   `HostNotProbed`, `TopologyEpochStale`, `LocalityDowngraded`, `TrustBelowMin`,
   `NoMatchingRouteStep`, `SurfaceKindUnsupported`)

## Phase 2 — FSM guard (`crates/fabric-graph/src/lease_fsm.rs`)

P0: `try_transition(from: LeaseState, to: LeaseState) -> Result<(), TransitionError>`
P0: transition matrix is the unit test target
P0: `TransitionError` is a single-variant enum

## Phase 3 — Integration

P0: `crates/fabric-graph/src/lib.rs` re-exports the new module
P0: `crates/fabric-graph/tests/surface_admit.rs` — 4 acceptance tests
P0: `crates/fabric-graph/tests/surface_lease_fsm.rs` — 6 FSM tests

## Phase 4 — CLI surface

P0: extend `cmd/checker/main.go` with `surface` subcommand reading
   `surface-spec.json` + `route-plan.json` → emit `Decision` + `ReasonCode[]`
P0: 2 Go tests proving the Rust + Go decisions match for the same inputs

## Phase 5 — Lock and commit

P0: `cargo test --workspace` → 84 + 12 (Rust) + 7 + 2 (Go) = 105 total, 0 failures
P0: `cargo run --example verify_fixtures` still green (no fixture regressions)
P0: `python3 program/scripts/check_*.py` → 4/4 pass
P0: regen MANIFEST
P0: commit `feat(surface): spec 019 surface plane (PF-WP-015) — types + admit + FSM`
P0: `meta/PHENOTYPE_ARCHITECTURE.md` addendum for 2026-09-05

## WP DAG

```
000 (program baseline) ─► 010 (capability inventory) ─► 015 (route compiler) ─► 019 (surface plane, R1) ─► 020 (route failover, R1)
                                              └─► 016 (NVMS adapter) ───────┘
                                              └─► 018 (checker, Go) ────────┘
```

## Critical-path note

Spec 019 MUST land before ADR-0030 (route failover) and before any R1
work on `fabric-workspace` (the lease FSM there depends on this contract).
