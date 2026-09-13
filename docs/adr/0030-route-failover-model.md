# ADR-0030 — Route Failover Model (PF-WP-021, R1)

**Status:** Accepted (impl landed 2026-09-06; see crates/fabric-graph/src/failover.rs)
**Date:** 2026-09-05
**Deciders:** fabric-runtime

## Context

PF-WP-020 (R0) produces a static `RoutePlan`. PF-WP-021 (R1) needs a
*dynamic* layer: when a `RouteStep` becomes invalid (host down, host
decommissioned, capability revoked, link degraded below `LatencyBudget`),
Fabric must re-plan or fail-over, not silently serve a broken route.

The Surface Plane (spec 019) provides the user-facing contract; this ADR
provides the runtime behavior under the surface.

## Decision

### 1. Failure triggers (deterministic, observable)

| Trigger | Source | Latency |
|:--|:--|:--|
| Heartbeat miss | `fabric_heartbeat: missing for > ttl_heartbeat * 3` | 3 × ttl (default 3s) |
| Capability revocation | `CapabilityDescriptor::epoch` decreased OR `topology_hash` changed | next plan cycle |
| Link degraded | `LinkMetrics.effective_bandwidth(max_latency_us) == None` | next plan cycle |
| Explicit revocation | `SurfaceLease::revoke(grace_period: Duration)` | grace_period (default 5s) |
| Stale topology | `TopologyEpoch` mismatch between plan and live | next plan cycle |

All triggers emit a `RouteStepFailure { step: RouteStep, reason: ReasonCode, at_unix: u64 }`
event onto the in-process `EventBus`. No silent failures.

### 2. Failover policy (per trigger)

| Trigger | Soft (AdmitWithNotes) | Hard (Reject + re-plan) |
|:--|:--|:--|
| Heartbeat miss | — | yes, immediately |
| Capability revocation | — | yes, immediately |
| Link degraded (latency_budget) | — | yes, immediately |
| Link degraded (bandwidth) | yes (notes) if within 20%, else hard | yes |
| Explicit revocation | — | yes, after grace_period |
| Stale topology | yes (notes) if same epoch, else hard | yes |

Soft failures are recorded in the surface's notes; the surface stays
**active** but the next plan cycle may re-route. Hard failures move the
surface to `LeaseState::Revoked` immediately and require a fresh
`admit` call.

### 3. Lease semantics

- `Pending` → `Active` (acquire, monotonic timestamp)
- `Active` → `Revoked` (explicit revoke OR hard failure OR grace_period expired)
- `Active` → `Expired` (ttl_lease reached, default 24h)
- `Pending` → `Expired` (heartbeat miss before promote)
- No back-transitions. Once `Revoked` or `Expired`, the seat is permanently gone.

### 4. Re-plan strategy

`fabric-graph::planner::replan(plan: &RoutePlan, topology: &Topology, event: &RouteStepFailure) -> Option<RoutePlan>`

- If the failed step had an alternative `RouteStep` (lower score but same capability
  + locality class), re-route to it. Surface lease moves to the new step in
  `Active` state (no `Revoked` intermediate).
- If no alternative, surface goes to `Revoked` and the user is told to
  re-`admit`. This is the "fail loud" path.
- Re-plans are journaled to the workspace event log (spec 017 contract).

### 5. Backoff

- Re-plan attempts on the same `RouteStepFailure` use exponential backoff
  starting at 100ms, doubling up to 30s. After 5 attempts on a single
  step, surface is `Revoked`.
- Backoff is per-step, not per-plan. Independent step failures don't
  interfere with each other.

### 6. Observability

- Every failover emits a `RouteStepFailure` event AND a `LeaseState::Revoked`
  event. Both are append-only to the workspace event log.
- The event log is the source of truth for "why did this surface die?"
  forensics.

## Consequences

Positive:
- No silent failures. Every degraded surface produces a journaled event.
- Soft failures allow graceful degradation without re-admit.
- The re-plan strategy is a pure function (`replan(plan, topology, event) -> Option<RoutePlan>`),
  unit-testable, no I/O.

Negative:
- Adding a per-surface event log means `SurfaceLease` carries an
  `event_log: Vec<RouteStepFailure>` field. This is a small struct-size
  tax. Acceptable.
- The 5-attempt retry budget means a flaky link can spend 30s before
  being declared `Revoked`. Trade-off accepted in exchange for stability.

## Alternatives considered

1. **Fail-fast on any degraded state** — rejected. Real workloads
   tolerate degraded links for minutes; failing fast is anti-user.
2. **Always re-route to any compatible step, never revoke** — rejected.
   Hides hardware failure from operators.
3. **Single backoff across all surfaces** — rejected. Couples
   unrelated surfaces; a slow swap in one shouldn't delay swaps in
   others.

## Compliance

- This ADR is the contract that spec 019's `LeaseState` FSM enforces
  (`Phase 2 / S019-04`).
- This ADR is the contract that PF-WP-021 implementation
  (`fabric-graph::replan`) must satisfy.
- The Go `cmd/checker` does NOT need to implement failover in R1; it
  only needs the surface `admit` decision. Per ADR-0029, Rust is
  canonical for the dynamic layer.

## Open questions for R1 implementation

1. Where does the event bus live? (process-local, IPC, sidecar?)
2. Do we journal before or after state transition? (current decision: after, because journal must reflect truth, not intent)
3. Is `Revoked` terminal forever, or can the operator manually re-arm?
   (current decision: terminal; re-`admit` is the way back)

## Rollout

- ADR-0030 lands first
- Spec 019 (Surface Plane) lands second — the surface types this ADR
  manipulates are defined there
- PF-WP-021 implementation lands third — `replan` function + journaled
  event bus
- Tests for re-plan strategy (5–8 unit tests) land fourth
- Commit + MANIFEST regen
