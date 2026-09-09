# Plan 022 — Multi-Tenant Fairness Implementation

## Phase 0 (ADR-0028 — read before writing)

Read these files end-to-end BEFORE writing `leases_fairness.rs`:

1. `crates/fabric-graph/src/leases.rs` (current `rebind_or_fail`) — confirm
   `SurfaceLease` shape and `LeaseState::Pending` (we need to issue new
   leases from `pardon()`)
2. `crates/fabric-graph/src/surface.rs` — confirm `SurfaceSpec`,
   `SurfaceSpecError`, `SurfaceError` enums and how `new_lease()` is
   constructed
3. `crates/fabric-graph/src/surface_ops.rs` — confirm `new_lease()` is the
   public constructor and what `SurfaceSpecError` variants exist
4. `crates/fabric-graph/src/lib.rs` — confirm module registration
   conventions
5. `crates/fabric-graph/Cargo.toml` — confirm serde features available
6. `crates/fabric-graph/tests/surface_plane_integration.rs` — read for
   test scaffolding patterns (don't copy tests, learn the API surface)

Expected findings (likely but unverified):

- `SurfaceSpecError::EmptyName` is the only spec-validation variant; add
  `PardonError` as a separate top-level enum, not a variant
- `new_lease(spec) -> Result<SurfaceLease, SurfaceSpecError>` — use this
  for `pardon()`'s spec re-validation (defense in depth)
- `SurfaceLease::handle` is a `SurfaceHandle(uuid::Uuid)` — opaque,
  don't construct directly
- No existing `TenantId` type — define new
- `BTreeMap` is in std — no need for `indexmap`

## Phase 1 — Implementation

### 1.1 `leases_fairness.rs`

Type definitions in this order:

1. `TenantId(String)` — newtype wrapper, derives Hash/Ord for map keys
2. `FairnessPolicy` enum — 4 variants per spec §3
3. `FairnessDecision` + `DenyReason`
4. `TenantAccounting` struct
5. `FairnessSnapshot` struct
6. `FairnessQueue` impl with:
   - `new(policy)`
   - `try_acquire(tenant, weight) -> FairnessDecision`
   - `release(tenant, weight)`
   - `snapshot() -> FairnessSnapshot`

Algorithm for each policy:

- **Fifo**: rotate a `VecDeque<TenantId>`; pop from front, grant, push back
- **FairShare**: track deficit per tenant; pick tenant with max deficit
  (ties broken by FIFO); grant `weight`, update accounting
- **PriorityWeighted**: sort tenants by priority; within same priority,
  FIFO; deny if higher-priority tenant is waiting
- **WeightedRoundRobin**: each tenant has `weight` slots per rotation;
  cursor advances after each grant; refill rotation when all slots used

### 1.2 `leases.rs` addition — `pardon()`

Small additive function:

```rust
pub fn pardon(
    spec: SurfaceSpec,
    operator_token: &str,
) -> Result<SurfaceLease, PardonError>
```

- Validate `operator_token` against a fixed string for now
  ("ops:phenotype:default"). Production would verify an Ed25519
  signature per Q4-C spec.
- Call `new_lease(spec)` to re-validate the spec.
- Return the new lease in `Pending` state.

### 1.3 `lib.rs` additions

- `pub mod leases_fairness;`
- Re-export `FairnessPolicy`, `FairnessQueue`, `TenantId`, `FairnessSnapshot`

## Phase 2 — Tests

### 2.1 Unit tests (in `leases_fairness.rs`, `#[cfg(test)]`)

- T-F01: `try_acquire_fifo_round_robin` — A, B, C, A, B, C, ...
- T-F02: `try_acquire_fair_share_equal_grants` — FairShare{weight=3},
  A,B,C each request 3 → 3 grants round-robin
- T-F03: `try_acquire_priority_skips_higher` — A=priority 1, B=priority 2;
  A served first
- T-F04: `try_acquire_wrr_weighted_slots` — A=weight 2, B=weight 1; A gets
  twice as many grants before B
- T-F05: `release_updates_accounting` — grant A 5, release A 3,
  snapshot reflects
- T-F06: `snapshot_serde_round_trip` — snapshot → JSON → snapshot
- T-F07: `pardon_valid_token_returns_pending_lease`
- T-F08: `pardon_bad_token_rejected`

### 2.2 Integration tests (`tests/lease_fairness_integration.rs`)

- T-FI-01: `multi_tenant_stress_1000_ops` — 5 tenants, 1000 ops,
  invariants hold (deficit never exceeds request, no starvation)
- T-FI-02: `wire_format_snapshot_round_trip` — full snapshot
  JSON round-trip
- T-FI-03: `pardon_creates_independent_lease` — pardoned lease has
  fresh handle, fresh history, original revoked lease unchanged
- T-FI-04: `policy_switch_preserves_accounting` — change policy
  mid-flight, accounting state persists
- T-FI-05: `deficit_convergence_under_release` — A requests 100,
  granted 30, releases 30 → deficit returns to 0

## Phase 3 — Verification

- `cargo test -p fabric-graph --lib leases_fairness::` → 8/8 pass
- `cargo test -p fabric-graph --test lease_fairness_integration` → 5/5 pass
- `cargo test --workspace` → no regressions
- 4/4 spec checks green
- MANIFEST regen

## Phase 4 — Commit

Single feat commit: `feat(leases): multi-tenant fairness + pardon() (PF-WP-022 v2, spec 022, R1 closeout)`.

## Risk register

- **Risk**: `SurfaceSpec` changes in a future spec might break `pardon()`
  contract. **Mitigation**: `pardon` re-validates via `new_lease()` —
  the lease is constructed from a freshly-validated spec, not by
  resurrecting an old one.
- **Risk**: PriorityWeighted starvation for low-priority tenants under
  sustained high-priority load. **Mitigation**: out of scope for R1
  closeout; documented in spec §2.
- **Risk**: BTreeMap iteration order for ties is alphabetical by TenantId,
  which may not match FIFO. **Mitigation**: track an explicit
  insertion-order counter for tie-breaking in FairShare.
