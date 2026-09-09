# Spec 022 — Lease Multi-Tenant Fairness (PF-WP-022 v2, R1 closeout)

**Status:** draft
**WP:** PF-WP-022-v2
**Release gate:** R1 closeout (closes the last 5% of R1)
**Depends on:** [specs/020-route-lease-integration](../020-route-lease-integration/spec.md), [specs/019-surface-plane](../019-surface-plane/spec.md)
**Enables:** PF-WP-050/060 (audio/video runtime), specs/017 (workspace persistence — fairness audit log)

## 1. Background

Spec 020 (`leases::rebind_or_fail`) handles the *single-tenant* case: one
lease, one re-bind or fail decision. But the R2 surface-plane runtime
needs to answer a different question: **when many tenants contend for
the same set of host resources (RT islands, GPUs, audio channels), how
do we decide who gets capacity next?**

Today there is no answer — the leases module treats all leases as
first-class and doesn't know about tenants. This spec adds:

1. A `FairnessPolicy` enum (4 variants) describing how capacity is divided
2. A `FairnessQueue` that tracks per-tenant allocation across many
   `try_acquire` / `release` cycles
3. An out-of-band `lease::pardon()` operator API for rescuing
   `LeaseState::Revoked` leases (Q4 recommendation C, kept out of FSM)

The 5% of R1 that's left is precisely the items this spec delivers:

- multi-tenant fairness (R3 explicit, brought into R1 closeout per
  operator direction "finish that 5% aggressively")
- the Q4 `lease::pardon()` operator escape hatch
- the four open questions (Q1-Q4) from `releases/2026-09-08-R1.md:184`
  answered and pinned in code

## 2. Scope

**In:**

- `fabric_graph::leases_fairness::FairnessPolicy` — 4 variants:
  - `Fifo` (no weights, round-robin)
  - `FairShare { weight }` (per-tenant weight, deficit-based)
  - `PriorityWeighted { priority }` (u8 priority, 0=highest)
  - `WeightedRoundRobin { weight }` (RR proportional to weight)
- `fabric_graph::leases_fairness::FairnessQueue::new(policy)`
- `try_acquire(&mut self, tenant_id, requested_weight) -> FairnessDecision`
- `release(&mut self, tenant_id, weight)`
- `snapshot(&self) -> FairnessSnapshot` for audit log
- `fabric_graph::leases::pardon(handle, spec, operator_token) -> Result<SurfaceLease, PardonError>`
  (operator escape hatch from Q4-C)
- 8 unit tests in `leases_fairness.rs` + 5 integration tests in
  `tests/lease_fairness_integration.rs`

**Out:**

- Cross-tenant priority inheritance (parent → child tenants)
- Per-tenant microsecond budget enforcement
- Distributed fairness (multi-instance consensus)
- Wire-format for fairness decisions (Q2 deferred to R2)

## 3. Locked types

```rust
// leases_fairness.rs — read leases.rs + model.rs BEFORE editing (ADR-0028).

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

/// How capacity is divided across tenants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FairnessPolicy {
    /// Round-robin, no weights.
    Fifo,
    /// Each tenant gets weight units per rotation; deficit drives next pick.
    FairShare { weight: u32 },
    /// Priority ordering; lower number = higher priority.
    PriorityWeighted { priority: u8 },
    /// Round-robin scaled by weight.
    WeightedRoundRobin { weight: u32 },
}

/// A tenant's identifier (opaque string, e.g. "ui-window:abc123").
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TenantId(pub String);

/// Outcome of a try_acquire call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FairnessDecision {
    Granted { tenant: TenantId, granted_weight: u32, deficit_after: i64 },
    Denied { tenant: TenantId, reason: DenyReason, current_deficit: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenyReason {
    /// Tenant is over their allotment; deficit is positive.
    OverAllotment,
    /// Priority-based denial (higher-priority tenant should be served first).
    LowerPriority { blocking: TenantId, blocking_priority: u8 },
    /// Queue is at capacity and this tenant already has a slice.
    QueueFull,
}

/// Per-tenant accounting state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantAccounting {
    pub granted: u64,
    pub released: u64,
    pub deficit: i64,
    pub priority: u8,
}

/// Snapshot of the queue for audit logging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FairnessSnapshot {
    pub policy: FairnessPolicy,
    pub accounting: BTreeMap<TenantId, TenantAccounting>,
    pub total_granted: u64,
    pub total_released: u64,
}

/// The fairness queue itself.
pub struct FairnessQueue {
    policy: FairnessPolicy,
    accounting: BTreeMap<TenantId, TenantAccounting>,
    rotation: Vec<TenantId>, // for Fifo + WeightedRoundRobin
    total_granted: u64,
    total_released: u64,
}

impl FairnessQueue {
    pub fn new(policy: FairnessPolicy) -> Self;
    pub fn try_acquire(&mut self, tenant: TenantId, weight: u32) -> FairnessDecision;
    pub fn release(&mut self, tenant: TenantId, weight: u32);
    pub fn snapshot(&self) -> FairnessSnapshot;
}
```

```rust
// leases.rs addition — Q4-C operator escape hatch.

use crate::surface_ops::new_lease;

/// Operator-initiated rescue of a Revoked lease.
/// Creates a NEW SurfaceLease from the original spec; the revoked
/// lease is left in place (audit trail intact).
pub fn pardon(
    spec: SurfaceSpec,
    operator_token: &str, // signed operator identity, e.g. "ops:koosha@2026-09-08"
) -> Result<SurfaceLease, PardonError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PardonError {
    SpecInvalid(SurfaceSpecError),
    TokenRejected, // signature/identity verification failed
}
```

## 4. Algorithm

### 4.1 FairShare
- `deficit[tenant] = max(0, requested - granted)`
- Pick tenant with highest deficit; grant `weight` units.
- Ties broken by FIFO order of `release` calls.

### 4.2 PriorityWeighted
- Group tenants by `priority`.
- Within group, FIFO.
- Only consider lower-priority tenants when higher groups are empty.

### 4.3 Fifo
- Pure rotation order; no weights.

### 4.4 WeightedRoundRobin
- Each tenant gets `weight` slots per full rotation.
- Rotation cursor advances after each `try_acquire`.

## 5. Acceptance criteria

1. `FairnessQueue::new(FairnessPolicy::Fifo)` returns an empty queue.
2. With 3 tenants A, B, C and Fifo, try_acquire order is A, B, C, A, B, C, ...
3. With FairShare{weight=3} and tenants A,B,C all requesting 3, each gets
   exactly one grant before any gets a second.
4. With PriorityWeighted{priority=1} for A and {priority=2} for B, A is
   served first; only after A's slice is exhausted does B get served.
5. `release` decreases deficit and increments `total_released`.
6. `snapshot` round-trips through serde JSON.
7. `lease::pardon(spec, valid_token)` returns `Ok(SurfaceLease)` in
   `LeaseState::Pending`.
8. `lease::pardon(spec, "bad")` returns `Err(PardonError::TokenRejected)`.

## 6. Test plan

- 8 unit tests in `leases_fairness.rs` (cfg test): one per policy + serde
  round-trip + snapshot invariance + pardon success + pardon bad-token.
- 5 integration tests in `tests/lease_fairness_integration.rs`: end-to-end
  with multi-tenant stress (1000 ops, invariants hold), wire-format
  round-trip, pardon audit-log integration, policy switching, deficit
  convergence.

## 7. Rollout

1. Spec 022 lands (this doc).
2. `crates/fabric-graph/src/leases_fairness.rs` added with 8 unit tests.
3. `crates/fabric-graph/src/lib.rs` adds `pub mod leases_fairness;` and
   `pub use leases_fairness::*;` (selective re-exports).
4. `crates/fabric-graph/tests/lease_fairness_integration.rs` with 5 tests.
5. `crates/fabric-graph/src/leases.rs` adds `pardon()` (small addition).
6. `MANIFEST.sha256` regen.
7. Commit + WORKLOG + meta/ARCHITECTURE addenda.

This closes the last 5% of R1.
