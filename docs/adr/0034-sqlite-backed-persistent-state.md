# ADR-0034: SQLite-Backed Persistent State

## Status

Proposed

## Date

2026-09-12

## Context

R3 requires durable state across daemon restarts. The R0→R1→R2
progression used in-memory `Topology`, `SurfaceLease`, and `RoutePlan`
types that were lost on process exit. ADR-0026 proposed JSONL persistence
but the actual R3 implementation uses SQLite for:

1. Atomic writes (no partial state on crash)
2. Query capability (find leases by node, plans by epoch)
3. WAL mode for concurrent reads during writes
4. Schema migration support for forward-compatible upgrades

## Decision

Create `fabric-persist` crate with five modules:

1. **`topology.rs`** — `persist_topology()` / `load_topology()` /
   `update_topology_meta()` — serialize `Topology` nodes/edges/meta
   to SQLite tables. `TopologyMeta` (name, description, epoch) stored
   as a single row. Empty topology loads as `Topology::new()`.

2. **`leases.rs`** — `insert_lease()` / `update_lease_state()` /
   `get_lease()` / `get_leases_by_node()` / `expire_leases()` /
   `cleanup_old_history()` — full `SurfaceLease` lifecycle in SQLite.
   History entries stored as separate rows with `lease_id` foreign key.
   State transitions tracked via `LeaseState` enum serialization.

3. **`routes.rs`** — `insert_route_plan()` / `get_route_plan()` /
   `get_plans_by_epoch()` / `replace_route_plan()` / `expire_plans()`.
   Route plans stored with full `RouteStep` sequences. Expired plans
   soft-deleted (marked expired, not removed).

4. **`schema.rs`** — `initialize_schema()` / `run_migrations()` /
   `get_schema_version()`. Version-tracked schema with incremental
   migrations. `SCHEMA_VERSION` constant updated on schema changes.

5. **`recovery.rs`** — `recover_state()` loads topology + active leases
   + non-expired plans from SQLite. Returns `RecoveryResult` with counts
   and any warnings (orphaned plans, expired leases).

## Alternatives considered

### A. JSONL file (ADR-0026 original proposal)
Rejected: JSONL is append-only and requires full-file re-read on startup.
SQLite provides indexed queries, atomic updates, and WAL mode for
concurrent access. The JSONL format remains viable as an export format
but not as the primary store.

### B. PostgreSQL / external database
Rejected: the daemon is a single-process, single-node service. Adding
a PostgreSQL dependency for R3 is overkill. SQLite is embedded,
zero-config, and sufficient for the expected data volume (hundreds of
leases, not millions).

### C. sled / redb (Rust-native embedded databases)
Rejected: sled and redb are younger projects with smaller ecosystems.
SQLite has decades of production hardening, WAL mode, and广泛 tooling
(operator can inspect the database with `sqlite3` CLI). For R3, the
stability of SQLite outweighs the Rust-native advantage.

## Consequences

- `fabric-persist` is the canonical persistence layer.
- Schema versioning ensures forward-compatible upgrades.
- `recover_state()` is the single entry point for daemon startup.
- Expired leases/plans are soft-deleted, not hard-deleted — audit
  trail is preserved.
- 21 unit tests across all modules. All pass.

## References

- ADR-0026 (workspace persistence format) — original JSONL proposal,
  now superseded by SQLite for primary store.
- ADR-0033 (fabric daemon architecture) — daemon uses `fabric-persist`
  for all state management.
- Spec 020 (route lease integration) — `SurfaceLease` schema matches
  the lease lifecycle defined in spec 020.
- ADR-0030 (route failover model) — `expire_plans()` and
  `expire_leases()` implement the failover cleanup path.
