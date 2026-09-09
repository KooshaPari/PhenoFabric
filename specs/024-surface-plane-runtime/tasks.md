# Spec 024 — Tasks: Surface-plane runtime (PF-WP-030)

## Definition of done

- [ ] spec 024 meta/spec/plan/tasks authored
- [ ] `crates/fabric-graph/src/surface_runtime.rs` — `SurfaceRegistry` + `RegistryEntry` + `Invalidation` + `notify_node_failure`
- [ ] `crates/fabric-graph/src/surface_ops.rs` — additive `bind_with_topology(&mut lease, plan_id, step, topology)` that resolves the node from `topology.nodes` and rejects unknown nodes with `SurfaceError::UnknownNode`
- [ ] `crates/fabric-graph/src/lib.rs` — `pub mod surface_runtime;` + re-exports
- [ ] 5 unit tests in `surface_runtime.rs` (cfg test): insert/get/remove round-trip, notify_node_failure invalidates touching leases, notify returns invalidation list, bind_with_topology resolves endpoint, bind_with_topology rejects unknown node
- [ ] 5 integration tests in `tests/surface_runtime_integration.rs`: end-to-end bind+invalidate, multi-lease invalidate-only-touching, handle drop after invalidation, snapshot serde round-trip, notify with empty failed_nodes is noop
- [ ] `cargo test -p fabric-graph` — 0 regressions, +10 tests pass
- [ ] `cargo test --workspace` — 0 regressions
- [ ] `go test ./cmd/capprobe` — ok
- [ ] `go test ./cmd/checker` — ok (including -replan-binary integration)
- [ ] 4/4 spec checks green (manifest, schemas, openapi, links)
- [ ] `MANIFEST.sha256` regen for the new files
- [ ] WORKLOG.md append (2026-09-08 entry)
- [ ] meta/PHENOTYPE_ARCHITECTURE.md addendum
- [ ] Single commit per ADR-0028: `feat(surface): PF-WP-030 surface-plane runtime`
- [ ] R2 release doc (`releases/2026-09-XX-R2.md` or addendum to R1) marks R2 30%

## Sub-tasks

### S024-01 — Phase 0 read (ADR-0028)
- [ ] Read failover.rs end-to-end
- [ ] Read surface.rs end-to-end
- [ ] Read surface_ops.rs end-to-end
- [ ] Read lease_fsm.rs end-to-end
- [ ] Read model.rs end-to-end
- [ ] Read builder.rs end-to-end
- [ ] Read lib.rs end-to-end
- [ ] Document actual API shapes (Topology::nodes, Node::id, RouteStep::node, TopologyEpoch)

### S024-02 — spec 024 artifacts
- [ ] `specs/024-surface-plane-runtime/meta.json`
- [ ] `specs/024-surface-plane-runtime/spec.md`
- [ ] `specs/024-surface-plane-runtime/plan.md`
- [ ] `specs/024-surface-plane-runtime/tasks.md`
- [ ] `specs/INDEX.md` row 024

### S024-03 — SurfaceRegistry
- [ ] `SurfaceRegistry` struct with `HashMap<SurfaceHandle, RegistryEntry>`
- [ ] `RegistryEntry { handle, lease, spec }`
- [ ] `Invalidation { handle, reason }`
- [ ] `insert(handle, lease, spec) -> Result<(), RegistryError>`
- [ ] `get(&handle) -> Option<&RegistryEntry>`
- [ ] `remove(&handle) -> Option<RegistryEntry>`
- [ ] `len()`, `is_empty()`
- [ ] `notify_node_failure(failed_nodes: &[NodeId]) -> Vec<Invalidation>` — invalidates leases whose `lease.current_binding` touches any node in `failed_nodes`, with `LeaseExitReason::HostFailure{ host_node: first_failed_node }`

### S024-04 — bind_with_topology
- [ ] `bind_with_topology(&mut lease, plan_id, step, topology) -> Result<(), SurfaceError>` in surface_ops.rs
- [ ] Find `topology.nodes.iter().find(|n| n.id == step.node)`
- [ ] On missing node: return `SurfaceError::UnknownNode { node: step.node.clone() }`
- [ ] Resolve endpoint via `derive_endpoint_for_step` (already exists; verify it handles the new flow)
- [ ] Delegate to existing `bind()`

### S024-05 — lib.rs
- [ ] Add `pub mod surface_runtime;` (alphabetical position before `surface_ops`)
- [ ] Add `pub use surface_runtime::{SurfaceRegistry, RegistryEntry, Invalidation};`

### S024-06 — Unit tests (cfg test)
- [ ] `registry_insert_get_remove_round_trip`
- [ ] `registry_notify_node_failure_invalidates_touching_leases`
- [ ] `registry_notify_node_failure_returns_invalidation_list`
- [ ] `bind_with_topology_resolves_endpoint`
- [ ] `bind_with_topology_rejects_unknown_node`

### S024-07 — Integration tests
- [ ] `tests/surface_runtime_integration.rs` (5 tests)
- [ ] Real `TopologyBuilder` + `compile()` to build bindings
- [ ] End-to-end with `SurfaceRegistry`

### S024-08 — Verification
- [ ] cargo test -p fabric-graph (lib + tests)
- [ ] cargo test --workspace
- [ ] go test ./cmd/capprobe + ./cmd/checker
- [ ] 4/4 spec checks
- [ ] MANIFEST regen

### S024-09 — Commit
- [ ] Single commit, WORKLOG, meta, MANIFEST

## Cross-dependencies

- Uses `crates/fabric-graph/src/surface.rs::SurfaceHandle` — already exists (spec 019)
- Uses `crates/fabric-graph/src/surface_ops.rs::bind` — already exists (spec 019)
- Uses `crates/fabric-graph/src/model.rs::Topology`, `NodeId`, `RouteStep`, `Node` — already exist
- Does NOT use `leases::rebind_or_fail` (spec 020) — that's the integration glue, separate
- Does NOT use `leases_fairness::pardon` (spec 022) — that's escape hatch, separate

## Out of scope

- `bind_with_topology` does NOT auto-fail leases on node failure (separate concern, R3)
- No replacement selector — spec 020 leases::rebind_or_fail does that
- No fairness integration — spec 022 already covers that
- No surface rotation rate-limiting — spec 022 R3 work
