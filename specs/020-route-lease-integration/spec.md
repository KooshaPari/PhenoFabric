# Spec 020 — Route Lease Integration (PF-WP-022, R1)

**Status:** draft
**WP:** PF-WP-022
**Release gate:** R1
**Depends on:** [specs/019-surface-plane](../019-surface-plane/spec.md), [adr/0030-route-failover-model.md](../../adr/0030-route-failover-model.md)
**Enables:** specs/017 (workspace persistence)

## 1. Background

The R1 release ships three pieces of the failover story in three separate
slices that have been committed across this and the prior session:

| Slice | Where | Commit |
|:--|:--|:--|
| Surface plane types + lease FSM | `crates/fabric-graph/src/{surface,surface_ops,lease_fsm,decision}.rs` | `dd0dafb` |
| Route failover module (`replan()`) | `crates/fabric-graph/src/failover.rs` | `b164efa` + ADR-0030 in `01333c6` |
| Checker blacklist flag | `cmd/checker/{main,checks,decision,checks_test}.go` | `93b30f4` |

Each of those slices is independently tested and proven. What they do NOT
do — and what spec 020 fixes — is *talk to each other through one
documented integration contract*.

Today a caller that wants "give me a surface, fail over when the host
dies" has to write the integration glue themselves:

1. Construct a `SurfaceLease` from a `SurfaceSpec` (spec 019).
2. Call `failover::replan()` when something fails (ADR-0030).
3. Interpret the `FailoverOutcome` and call `surface_ops::bind()` /
   `surface_ops::fail()` accordingly.
4. Wire the Go `cmd/checker -failover-blacklist` so the operator can
   pre-empt the failure.

That glue is exactly the kind of thing that drifts between call sites and
silently does the wrong thing — the explicit failure mode ADR-0030 was
written to prevent ("no silent failures").

## 2. Scope

In:

- `fabric_graph::leases::rebind_or_fail` — the single integration entry
  point that ties `failover::replan` to the surface plane.
- A documented mapping from `FailoverOutcome` → `SurfaceError` /
  `LeaseState` transition.
- Strict-epoch-binding enforcement: when `SurfaceSpec::strict_epoch_binding`
  is true and the topology epoch drifts between bind and re-bind, the
  surface is invalidated with `SurfaceError::EpochDrift` (already typed in
  `surface.rs`, this spec binds the call site).
- `cmd/checker -failover-blacklist` is the operator-facing CLI surface for
  the integration (already shipped at commit `93b30f4` — this spec ratifies
  the contract it now implements).
- End-to-end integration tests that exercise the full path with a real
  `TopologyBuilder` + `RoutePlan` + `SurfaceSpec` + `SurfaceLease`.

Out:

- Topology-driven full `replan()` from inside the Go checker (would need
  cgo or shelling to a Rust binary; deferred to R2).
- Multi-tenant fairness across surfaces (R3 per the cockpit — PF-WP-022
  v2).
- Trust-root / revocation CA (R2).
- Per-surface event log (ADR-0030 §6) — this spec emits the events but
  does not own the log persistence.

## 3. Locked types (read these from source before changing)

The integration entry point in
`crates/fabric-graph/src/leases.rs` (new module) MUST operate on the
exact shapes below. Read `crates/fabric-graph/src/{failover.rs, surface.rs,
surface_ops.rs, lease_fsm.rs, model.rs}` first per ADR-0028.

```rust
// leases.rs — read the four upstream modules BEFORE editing this file.

use crate::failover::{replan, FailoverError, FailoverOutcome};
use crate::surface::{
    LeaseExitReason, LeaseState, SurfaceError, SurfaceLease, SurfaceSpec,
};
use crate::surface_ops::{bind, fail};
use crate::model::{Intent, NodeId, RoutePlan, RoutePlanId, Topology};

/// Rebind or fail the lease, depending on whether `failover::replan`
/// could find a replacement route on the (caller-pruned) post-failure
/// topology.
///
/// * On `FailoverOutcome::Replaced(new_plan)`:
///   - If `lease.spec.strict_epoch_binding` is true AND the
///     `RoutePlan::epoch` has drifted from the lease's prior `bound_at_epoch`,
///     return `SurfaceError::EpochDrift { previous, current }` (the surface
///     must be invalidated; no silent re-bind).
///   - Otherwise, call `surface_ops::bind(&mut lease, new_plan.id, new_step)`
///     to re-bind the lease to the new step. The user's `SurfaceHandle`
///     is unchanged; the prior binding moves to `lease.history`.
///   - The lease remains in `LeaseState::Active`.
/// * On `FailoverOutcome::NoReplacement`:
///   - Call `surface_ops::fail(&mut lease, LeaseExitReason::HostFailure {
///       host_node: first_failed_node })`. The lease transitions to
///       `LeaseState::Failed` and the caller (UI, workspace, agent) is
///       expected to drop the `SurfaceHandle` and re-admit if desired.
/// * On `Err(FailoverError::*)`:
///   - The error is bubbled up. The lease is unchanged.
///
/// `failed_nodes` is the list of `NodeId`s that were pruned from the
/// topology before this call. It is informational (used to populate
/// `LeaseExitReason::HostFailure`) — the topology passed in is the
/// post-failure topology.
pub fn rebind_or_fail(
    lease: &mut SurfaceLease,
    plan_id: RoutePlanId,
    new_step: crate::model::RouteStep,
    post_failure_topology: &Topology,
    intent: &Intent,
    old_plan: &RoutePlan,
    failed_nodes: &[NodeId],
) -> Result<RebindOutcome, SurfaceError>;
```

The result type:

```rust
pub enum RebindOutcome {
    /// The lease was silently re-bound (handle unchanged, current binding
    /// rotated into history).
    Rebound { new_plan_id: RoutePlanId },
    /// No replacement was available; the lease is now `Failed` and the
    /// caller MUST drop the handle.
    Failed { reason: LeaseExitReason },
}
```

The strict-epoch check uses the existing
`SurfaceError::EpochDrift { previous: u64, current: u64 }` variant
already declared in `surface.rs:300`.

## 4. Integration with `cmd/checker -failover-blacklist`

The Go checker does not currently call `rebind_or_fail` (no cgo, no Rust
binary to shell to — that is the R2 work). What it DOES do — and what
spec 020 ratifies — is the operator-facing pre-emption path:

```
checker -descriptor host.json -manifest app.json \
        -failover-blacklist host-1,host-3
```

When the host's `NodeID` appears in the blacklist, the checker returns
`DecisionReject` with `Finding { code: ReasonBlacklisted, severity:
SeverityBlock }` *before* any resource comparison. This is the
single-host decision equivalent of what `rebind_or_fail` returns when
`failover::replan` produces `NoReplacement`: at the level the checker
operates (no topology available), a blacklisted host is one we cannot
place on.

The two paths must agree on the contract:

| Layer | What "this node is failed" means |
|:--|:--|
| `cmd/checker` (operator) | Reject with `ReasonBlacklisted` before resource checks. |
| `rebind_or_fail` (runtime) | `failover::replan` returns `NoReplacement` → lease goes to `Failed`. |

This is the integration ADR-0030 §2 requires: "hard failures move the
surface to `LeaseState::Revoked` immediately". `ReasonBlacklisted` is
the operator-induced equivalent of the runtime-induced `HostFailure`
reason, and they both terminate the lease.

## 5. Acceptance criteria

1. `fabric_graph::leases::rebind_or_fail` exists and compiles with the
   signature shown in §3.
2. Given a `SurfaceLease` in `LeaseState::Active` bound to node `a`, a
   post-failure topology that contains only node `b`, and an intent that
   can be satisfied on either node:
   - `rebind_or_fail(lease, ..., &[a])` returns `RebindOutcome::Rebound` with
     `new_plan_id != old_plan.id`. The lease remains `Active`. The prior
     binding (`a`) is in `lease.history`. The handle is unchanged.
3. Given a `SurfaceLease` bound to node `a` (only node), and a
   post-failure topology that is empty:
   - `rebind_or_fail(lease, ..., &[a])` returns
     `RebindOutcome::Failed { reason: LeaseExitReason::HostFailure { host_node: a } }`.
   - The lease is now `LeaseState::Failed`. `lease.exit_reason` is `Some(...)`.
   - `is_terminal(lease.state) == true`.
4. Given a `SurfaceLease` with `strict_epoch_binding = true`, bound at
   `epoch = 5`, and a `new_plan` with `epoch = 7`:
   - `rebind_or_fail` returns `SurfaceError::EpochDrift { previous: 5,
     current: 7 }`. The lease is unchanged.
5. `cmd/checker -failover-blacklist host-x` against a host whose
   `NodeID = host-x` returns `DecisionReject` with `ReasonBlacklisted`
   severity `Block`. Already implemented and tested (3 new tests in
   `cmd/checker/checks_test.go`); spec 020 ratifies the contract.

## 6. Test plan (must be green before commit)

- `crates/fabric-graph/src/leases.rs` — 5 unit tests covering each
  acceptance criterion above.
- `crates/fabric-graph/tests/lease_integration.rs` — 4 end-to-end tests
  exercising `rebind_or_fail` with a real `TopologyBuilder`, `RoutePlan`,
  and `SurfaceLease`.
- `cmd/checker/checks_test.go` — 3 existing tests verify the
  `-failover-blacklist` contract (already in place; spec 020 just pins
  them as the acceptance evidence).

## 7. Rollout

- Spec 020 lands first (this doc).
- `crates/fabric-graph/src/leases.rs` is added (untracked, like spec 019
  was — read the 4 upstream modules first per ADR-0028; the 50-error
  cascade that bit spec 019's stub source must not repeat).
- `crates/fabric-graph/src/lib.rs` adds `pub mod leases;` next to the
  other R1 modules.
- 5 unit + 4 integration tests land and pass.
- 4/4 spec checks remain green; MANIFEST is regen'd.
- Commit + WORKLOG addendum + meta/PHENOTYPE_ARCHITECTURE.md addendum.

The actual Rust implementation of `rebind_or_fail` is the next R1
wedge. This spec is the contract that implementation must satisfy.
