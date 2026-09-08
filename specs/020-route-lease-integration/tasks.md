# Tasks — Spec 020 Route Lease Integration (PF-WP-022, R1)

| ID | Sub-task | Definition of done |
|:--|:--|:--|
| S020-01 | `RebindOutcome` enum (2 variants) | Compiles, derives `Debug`, `PartialEq`, JSON-serializable for event-log use |
| S020-02 | `rebind_or_fail` signature matches spec §3 | Compiles with the exact 7-param signature; return type `Result<RebindOutcome, SurfaceError>` |
| S020-03 | strict-epoch pre-check | Returns `SurfaceError::EpochDrift { previous, current }` BEFORE calling `failover::replan` when `strict_epoch_binding=true` and epoch drifted |
| S020-04 | On `Replaced` outcome → silent re-bind | Calls `surface_ops::bind(&mut lease, new_plan.id, step)`; handle unchanged; prior binding moves to `lease.history`; returns `RebindOutcome::Rebound { new_plan_id }` |
| S020-05 | On `NoReplacement` outcome → lease Failed | Calls `surface_ops::fail(&mut lease, LeaseExitReason::HostFailure { host_node })`; returns `RebindOutcome::Failed { reason }` |
| S020-06 | On `Err(FailoverError)` → bubble up | Returns the underlying `SurfaceError` (wrap FailoverError → SurfaceError); lease unchanged |
| S020-07 | 5 unit tests in `leases.rs` `#[cfg(test)]` | T-L01..L05 all green |
| S020-08 | 4 integration tests in `tests/lease_integration.rs` | T-I01..I04 all green; use real `TopologyBuilder` + `RoutePlan` + `SurfaceLease` |
| S020-09 | `pub mod leases;` added to `crates/fabric-graph/src/lib.rs` | `cargo check -p fabric-graph` clean |
| S020-10 | `cargo test --workspace` clean | 118 Rust + 0 fail baseline preserved (or +9 from this spec) |
| S020-11 | Go checker `-failover-blacklist` tests pass | `cmd/checker/checks_test.go` 3 blacklist tests green (already shipped; spec 020 ratifies) |
| S020-12 | 4/4 spec checks | manifest ✓ · schemas ✓ · openapi ✓ · links ✓ |
| S020-13 | Commit + WORKLOG + meta addendum | commit message includes `feat(leases): spec 020 route lease integration (PF-WP-022)`; both addenda appended |

## Cross-deps

- S020-02 depends on S020-01
- S020-04 depends on S020-02, S020-03 (the strict-epoch check is a guard)
- S020-05 depends on S020-02
- S020-06 depends on S020-02
- S020-07 depends on S020-04, S020-05, S020-06
- S020-08 depends on S020-07
- S020-10 depends on S020-08, S020-09
- S020-13 depends on S020-10, S020-11, S020-12

## Definition of done (overarching)

- All 13 sub-tasks green
- Spec 020 status moves from `draft` to `accepted`
- Manifest regen + 4/4 spec checks green
- Commit + meta/PHENOTYPE_ARCHITECTURE.md addendum

## Honest notes for the next session

- The integration spec is the contract. Implementation of
  `crates/fabric-graph/src/leases.rs` is the next R1 wedge but is NOT
  this turn's deliverable — this turn ships spec 020 + the contract it
  defines for the Rust + Go halves.
- The Rust implementation MUST start with reading the 6 source files
  listed in `plan.md` §Phase 0 *end-to-end in the same session*. The
  cascade that bit spec 019's first stub is the direct failure mode if
  this step is skipped.
- `cmd/checker -failover-blacklist` is already implemented (this
  session's commit `93b30f4`). Spec 020 ratifies it as the operator-
  facing half of the integration; the runtime-facing half
  (`rebind_or_fail`) is the spec's R1 deliverable for the next wedge.
