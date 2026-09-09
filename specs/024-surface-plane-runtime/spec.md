# Spec 024 — Surface-plane runtime (PF-WP-030, R2 wedge)

## Goal

Provide a runtime registry for active `SurfaceLease`s so the failover hook
has a place to invalidate leases when a node fails, and so the wire
transport (PF-WP-040) can enumerate active surfaces.

This is the R2 third wedge. Scope is intentionally minimal:

1. `SurfaceRegistry` — in-memory map `SurfaceHandle → (SurfaceLease, SurfaceSpec)`
2. `notify_node_failure(failed_nodes)` — invalidates leases whose current binding
   touches any failed node, returns the list of invalidated handles
3. `bind_with_topology()` — additive in `surface_ops`, replaces the
   `derive_endpoint_for_step` placeholder with a real topology lookup

## Public API

```rust
// In fabric_graph::surface_runtime
pub struct SurfaceRegistry {
    inner: HashMap<SurfaceHandle, RegistryEntry>,
}

pub struct RegistryEntry {
    pub handle: SurfaceHandle,
    pub lease: SurfaceLease,
    pub spec: SurfaceSpec,
}

pub struct Invalidation {
    pub handle: SurfaceHandle,
    pub reason: LeaseExitReason,
}

impl SurfaceRegistry {
    pub fn new() -> Self;
    pub fn insert(&mut self, handle: SurfaceHandle, lease: SurfaceLease, spec: SurfaceSpec);
    pub fn remove(&mut self, handle: &SurfaceHandle) -> Option<RegistryEntry>;
    pub fn get(&self, handle: &SurfaceHandle) -> Option<&RegistryEntry>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn handles(&self) -> Vec<SurfaceHandle>;
    pub fn notify_node_failure(&mut self, failed_nodes: &[NodeId]) -> Vec<Invalidation>;
}

// In fabric_graph::surface_ops (additive)
pub fn bind_with_topology(
    lease: &mut SurfaceLease,
    plan_id: RoutePlanId,
    new_step: RouteStep,
    topology: &Topology,
) -> Result<(), SurfaceError>;
```

## Semantics

### `SurfaceRegistry`

- Single-threaded, in-memory; no persistence (R3)
- `insert` overwrites if handle already present (idempotent)
- `remove` returns the entry (caller is responsible for dropping the handle
  if the lease transitioned to terminal)
- `notify_node_failure` returns the list of invalidations performed —
  caller is responsible for any external notification (e.g. event log
  emission, Go-side handle drop)

### `bind_with_topology`

- Validates that `new_step.node` exists in `topology.nodes` (else `SurfaceError::UnknownNode`)
- Validates that `new_step` satisfies `lease.spec` (locality, trust, capability) — else `SurfaceError::SpecViolation`
- Calls the existing `bind()` (now `bind_inner`) with the resolved `CapabilityEndpoint`
- Records `binding.bound_at_epoch = topology.epoch` (replacing the `0` placeholder)

## Acceptance criteria

1. `cargo test -p fabric-graph --lib surface_runtime::` — 5 unit tests pass
2. `cargo test -p fabric-graph --test surface_runtime_integration` — 5 integration tests pass
3. `cargo test --workspace` — all existing tests still pass (no regressions)
4. `cmd/checker -replan-binary` integration tests still pass (no breakage of R2 wedge #2)
5. `check_manifest.py` + `check_json_schemas.py` + `check_openapi.py` + `check_links.py` — 4/4 pass

## Open questions for R2

1. Multi-tenant: does the registry key by `SurfaceHandle` only or by
   `(SurfaceHandle, TenantId)`? — R3
2. Wire transport: should `notify_node_failure` emit a wire event, or
   is the Go side responsible? — R2 wedge #4 (PF-WP-040)
3. Persistence: when do we serialize the registry to disk? — R3

## Refs

- ADR-0028 (read-authoritative-source-first)
- ADR-0030 (route-failover model)
- specs/019-surface-plane (PF-WP-015, the surface types)
- specs/020-route-lease-integration (PF-WP-022, the rebind_or_fail contract)
