# Plan — Spec 020 Route Lease Integration (PF-WP-022)

## Phase 0 — Read sources (mandatory; ADR-0028 + WORKLOG rule)

Before opening any file under `crates/fabric-graph/src/leases.rs`, read
these in the same session — *do not* skim, read end-to-end:

1. `crates/fabric-graph/src/failover.rs` — `FailoverOutcome`, `FailoverError`,
   `replan()` signature, test patterns.
2. `crates/fabric-graph/src/surface.rs` — `SurfaceLease`, `LeaseState`,
   `LeaseExitReason`, `SurfaceError::EpochDrift`, `SurfaceSpec::strict_epoch_binding`.
3. `crates/fabric-graph/src/surface_ops.rs` — `bind`, `fail`, `is_terminal`,
   `new_lease`; the FSM guards.
4. `crates/fabric-graph/src/lease_fsm.rs` — `can_transition`, `next_state`
   for the table this spec depends on.
5. `crates/fabric-graph/src/model.rs` — `Intent`, `RoutePlan`,
   `RoutePlan::epoch` field, `RoutePlanId`, `RouteStep`.
6. `crates/fabric-graph/src/lib.rs` — re-exports the 4 modules above;
   the new module sits beside them.

Do NOT begin Phase 1 until all six are read. The cascade that bit
spec 019's first stub (3 errors from a 1-line lifetime fix) was the
direct consequence of skipping this step.

## Phase 1 — Module + types (`crates/fabric-graph/src/leases.rs`)

P0: `RebindOutcome` enum (2 variants: `Rebound { new_plan_id }`,
   `Failed { reason: LeaseExitReason }`)
P0: `rebind_or_fail(lease, plan_id, new_step, post_failure_topology,
   intent, old_plan, failed_nodes) -> Result<RebindOutcome, SurfaceError>`
P0: strict-epoch check BEFORE calling `failover::replan` (saves a
   needless compile() work when the binding would be invalidated anyway)
P0: on `FailoverOutcome::Replaced(new_plan)`:
   - if `lease.spec.strict_epoch_binding` && `new_plan.epoch != prior_epoch`:
     return `SurfaceError::EpochDrift { previous, current }`
   - else: `surface_ops::bind(&mut lease, new_plan.id, step)` and return
     `RebindOutcome::Rebound { new_plan_id: new_plan.id }`
P0: on `FailoverOutcome::NoReplacement`:
   - `surface_ops::fail(&mut lease, LeaseExitReason::HostFailure {
     host_node: failed_nodes.first().cloned().unwrap_or_default() })`
   - return `RebindOutcome::Failed { reason }`
P0: on `Err(FailoverError::*)`:
   - bubble up (lease unchanged)

## Phase 2 — Unit tests (5, in the same file under `#[cfg(test)]`)

| # | Test | Maps to acceptance |
|:--|:--|:--|
| T-L01 | `rebind_replaces_binding_when_replan_succeeds` | AC #2 |
| T-L02 | `rebind_returns_failed_when_replan_has_no_replacement` | AC #3 |
| T-L03 | `rebind_returns_epoch_drift_when_strict_binding_and_epoch_advanced` | AC #4 |
| T-L04 | `rebind_propagates_failover_error` | robustness |
| T-L05 | `rebind_outcome_round_trip_via_serde` | debuggability |

Each test must construct a real `Topology` via `TopologyBuilder`,
compile a real `RoutePlan`, build a real `SurfaceLease` via
`new_lease`, then call `rebind_or_fail` and assert.

## Phase 3 — Integration tests (`crates/fabric-graph/tests/lease_integration.rs`)

4 tests:

| # | Test | Description |
|:--|:--|:--|
| T-I01 | `failover_blacklist_drives_rebind_to_other_node` | 3-node topology, lease on node A, blacklist A → re-bind to B |
| T-I02 | `failover_blacklist_drives_rebind_to_no_replacement` | 1-node topology, blacklist that node → Failed |
| T-I03 | `failover_with_strict_epoch_binding_invalidates` | strict_epoch_binding=true, epoch drift → EpochDrift |
| T-I04 | `rebind_preserves_surface_handle` | handle is unchanged before/after successful re-bind |

## Phase 4 — Wire `pub mod leases;` in `crates/fabric-graph/src/lib.rs`

P0: alongside `pub mod failover;` (line ~35)

## Phase 5 — Lock and commit

P0: `cargo test -p fabric-graph --test lease_integration` → 4 tests pass
P0: `cargo test -p fabric-graph --lib leases` → 5 tests pass
P0: `cargo test --workspace` → 118 Rust + 0 fail (current baseline)
P0: `go test -count=1 ./... cmd/capprobe` → 6 (cached)
P0: `go test -count=1 ./... cmd/checker` → 12 (cached; -failover-blacklist
   tests are the operator-facing half of this spec's contract)
P0: `python3 program/scripts/check_*.py` → 4/4 pass
P0: regen MANIFEST for new file + lib.rs change
P0: commit `feat(leases): spec 020 route lease integration (PF-WP-022)`
P0: `meta/PHENOTYPE_ARCHITECTURE.md` addendum for 2026-09-08

## WP DAG (relevant R1 subset)

```
PF-WP-015 (surface plane, ✓) ─┐
                              ├─> PF-WP-022 (route lease integration, ◐ spec'd)
PF-WP-021 (failover, ✓) ─────┘                       │
                                                       v
                                              PF-WP-017 (workspace persistence, R1)
                                                       │
                                                       v
                                              PF-WP-022 v2 (multi-tenant fairness, R3)
```

## Critical-path note

Spec 020 is the contract that PF-WP-017 (workspace persistence) blocks
on. Without spec 020 there is no documented way for a workspace event
log to receive `LeaseState::Failed` events from the runtime side of a
failover — the workspace would have to invent the wire format, which is
exactly the silent-divergence failure mode ADR-0030 §6 warns against.

## Definition of done

- All 13 sub-tasks above green
- Spec 020 status moves from `draft` to `accepted`
- MANIFEST regen + 4/4 spec checks green
- Commit + meta/PHENOTYPE_ARCHITECTURE.md addendum + WORKLOG entry
