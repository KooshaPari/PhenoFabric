# Spec 024 — Plan: Surface-plane runtime (PF-WP-030)

## Phase 0 — Read authoritative source first (ADR-0028)

Read end-to-end before opening `surface_runtime.rs`:

1. `crates/fabric-graph/src/failover.rs` — `FailoverOutcome::{Replaced, NoReplacement}` shape, `prune()` semantics
2. `crates/fabric-graph/src/surface.rs` — `SurfaceHandle`, `SurfaceLease`, `SurfaceSpec`, `LeaseState`, `LeaseExitReason`
3. `crates/fabric-graph/src/surface_ops.rs` — `bind`, `complete`, `fail`, `revoke`, `expire`, `derive_endpoint_for_step` placeholder
4. `crates/fabric-graph/src/lease_fsm.rs` — `can_transition` / `next_state`
5. `crates/fabric-graph/src/model.rs` — `RouteStep`, `Node`, `NodeId`, `Topology`, `TopologyEpoch`, `CapabilityEndpoint`
6. `crates/fabric-graph/src/builder.rs` — `TopologyBuilder::find_node` (if it exists)
7. `crates/fabric-graph/src/lib.rs` — module declarations and `pub use` re-exports

Discovered shape (verified):
- `Topology::nodes: Vec<Node>` (not a HashMap; linear scan OK for small graphs)
- `Node::id: NodeId` (public field)
- `RouteStep::node: NodeId` (public field; no Node object)
- `TopologyEpoch(pub u64)` tuple struct
- `CapabilityEndpoint::new(node_id, address)` (verified from surface.rs)
- `SurfaceHandle::as_str() -> &str` for logging

## Phase 1 — Implement (additive, no breakage)

### Step 1: `surface_runtime.rs`

```rust
pub struct SurfaceRegistry { inner: HashMap<SurfaceHandle, RegistryEntry> }
pub struct RegistryEntry { handle, lease, spec }
pub struct Invalidation { handle, reason }

// Notify: scan all entries, for each entry whose lease.current_binding.node
// is in failed_nodes, fail() it with HostFailure reason, record invalidation,
// remove from map.
```

### Step 2: `surface_ops::bind_with_topology`

```rust
// Find topology.nodes.iter().find(|n| n.id == new_step.node) → SurfaceError::UnknownNode
// Resolve CapabilityEndpoint from node metadata → call inner bind()
```

### Step 3: `lib.rs`

Add `pub mod surface_runtime;` and re-export `SurfaceRegistry, RegistryEntry, Invalidation`.

## Phase 2 — Tests

### Unit tests (5)

1. `registry_insert_get_remove_round_trip`
2. `registry_notify_node_failure_invalidates_touching_leases`
3. `registry_notify_node_failure_returns_invalidation_list`
4. `bind_with_topology_resolves_endpoint`
5. `bind_with_topology_rejects_unknown_node`

### Integration tests (5)

1. `surface_runtime_end_to_end_bind_then_invalidate`
2. `surface_runtime_multi_lease_invalidates_only_touching`
3. `surface_runtime_handle_drop_after_invalidation`
4. `surface_runtime_snapshot_serde_round_trip`
5. `surface_runtime_notify_empty_failed_nodes_is_noop`

## Phase 3 — Verify

- `cargo test -p fabric-graph --lib` — 0 regressions
- `cargo test -p fabric-graph --test surface_runtime_integration` — 5 pass
- `cargo test --workspace` — 0 regressions
- 4/4 spec checks pass

## Phase 4 — Commit

- Single commit: `feat(surface): PF-WP-030 surface-plane runtime`
- WORKLOG append
- meta ARCHITECTURE addendum
- MANIFEST.sha256 regen

## Risk register

- Risk: `Topology::nodes` linear scan is O(N). For 10k-node topologies this matters. → R3 perf concern, not R2.
- Risk: `notify_node_failure` doesn't atomically fail + remove — caller must observe both. → Document in spec §Semantics.
- Risk: `bind_with_topology` rejects valid topologies that lack the node due to a missing field. → Catch by integration test 1.

## Phase 0 (ADR-0028) read order

failover → surface → surface_ops → lease_fsm → model → builder → lib (alphabetical within layer)
