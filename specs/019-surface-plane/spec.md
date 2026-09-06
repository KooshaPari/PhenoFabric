# Spec 019 — Surface Plane (PF-WP-015, R1)

## 1. Background

PF-WP-020 (R0, committed) produces a `RoutePlan`: an ordered list of `RouteStep`s,
each step binding a `NodeId` to a `CapabilityRef`, scored and ordered. That gets
you a "where to put a workload" decision. It does not get you a user-visible
surface — a window, a seat, an audio endpoint, a display, an input device.

R1 promotes Fabric from "compute placement" to "user-facing surface". The
Surface Plane is the layer that takes a `RoutePlan` and binds a `Surface`
(Seat / Window / Display / AudioSink / InputDevice) to one of its `RouteStep`s
under a `SurfaceLease`. The lease is what makes a surface hot-rebindable
when the underlying host fails (PF-WP-021, R1 — Route Failover).

## 2. Scope

In:
- `SurfaceSpec` (declarative: kind, parent topology id, requested locality, trust)
- `RouteBinding` (links a `SurfaceSpec` to a `RouteStep`)
- `SurfaceLease` (3-state FSM: Pending → Active → Revoked or Expired)
- `fabric::surface::admit` decision (reuses PF-WP-011's Decision/ReasonCode contract)
- `cmd/checker` extended with a `surface` subcommand

Out:
- Actual seat/window/audio backend (that is per-platform work in R2)
- Auth/identity (R2)
- Multi-tenant fairness (R3)

## 3. Surface types (locked types — read this from the source before changing)

These MUST be added in `crates/fabric-graph/src/surface.rs` per the read-authoritative-source-first
rule in WORKLOG + ADR-0028.

```rust
// surface.rs — read crates/fabric-graph/src/lib.rs and the model in this
// same directory BEFORE editing. Confirm NodeId, RouteStep, IntentId exist
// and have the fields shown here.

pub type SurfaceId = String;

pub enum SurfaceKind {
    Seat,        // interactive agent: window + input + audio out
    Window,      // display surface only
    Display,     // raw display, no input
    AudioSink,   // audio output endpoint
    AudioSource, // audio input (mic)
    InputDevice, // keyboard / mouse / tablet
    Storage,     // object-storage surface
    Network,     // logical port
}

pub struct SurfaceSpec {
    pub id: SurfaceId,
    pub kind: SurfaceKind,
    pub requested_locality: LocalityTier,
    pub min_trust: TrustLevel,
    pub tags: Vec<String>,
    pub parent_topology_epoch: TopologyEpoch,  // pin to a specific topology version
}

pub enum LeaseState {
    Pending,
    Active,
    Revoked,
    Expired,
}

pub struct SurfaceLease {
    pub surface_id: SurfaceId,
    pub state: LeaseState,
    pub bound_step: Option<RouteStep>,
    pub acquired_at_unix: u64,
    pub expires_at_unix: u64,
    pub trust_scope: TrustScope,
}

pub struct RouteBinding {
    pub surface_id: SurfaceId,
    pub step: RouteStep,
    pub score: ScoreBreakdown,
}
```

The `admit` function:

```rust
pub fn admit(spec: &SurfaceSpec, plan: &RoutePlan) -> Decision
```

Decision / Severity / ReasonCode come from `fabric-graph::decision` (a new
sub-module under `fabric-graph` for R1; the PF-WP-011 Go checker already has
its own types — see ADR-0029 for the parallel-impl policy).

## 4. Acceptance criteria

1. A `SurfaceSpec` with `requested_locality = L2CrossNumaShm` and a `RoutePlan`
   whose highest-scored `RouteStep` is `L0SameProcess` must emit
   `Decision::AdmitWithNotes` with reason `LOCALITY_DOWNGRADED`.
2. A `SurfaceSpec` whose `parent_topology_epoch` is older than the
   `Topology::epoch()` must emit `Decision::Reject` with reason
   `TOPOLOGY_EPOCH_STALE`.
3. A `SurfaceSpec` with `min_trust = Attested` and a `RoutePlan` whose
   `ScoreBreakdown.trust_score` is below threshold must emit
   `Decision::Reject` with reason `TRUST_BELOW_MIN`.
4. The `SurfaceLease` FSM must reject any transition other than
   `Pending→Active`, `Active→Revoked`, `Active→Expired`.

## 5. Test plan (must be green before commit)

- `crates/fabric-graph/tests/surface_admit.rs` — 4 tests, one per acceptance
  criterion above.
- `crates/fabric-graph/tests/surface_lease_fsm.rs` — 6 tests for the FSM
  transition matrix (4 valid + 2 invalid).
- `cmd/checker` extension: 2 tests proving the Go `surface` subcommand emits
  the same Decision/ReasonCode.

## 6. Rollout

PF-WP-015 spec lands first (this doc). PF-WP-021 Route Failover (ADR-0030)
binds against this spec; without this spec, ADR-0030 has no contract.
