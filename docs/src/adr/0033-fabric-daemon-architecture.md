# ADR-0033: Fabric Daemon Architecture

## Status

Proposed

## Date

2026-09-12

## Context

R3 requires a persistent service daemon that:
1. Manages topology, lease, and plan state across restarts
2. Handles wire transport connections per spec 025
3. Provides health/status endpoints for operators
4. Integrates with `fabric-persist` for SQLite-backed durability
5. Runs as a single process with graceful shutdown

Prior sessions (R2 wedge #3) attempted a surface-plane runtime that
hit the ADR-0028 stuck-loop pattern on aspirational APIs. The daemon
design grounds itself in the verified `fabric-graph` + `fabric-persist`
API surface.

## Decision

Create `fabric-daemon` as a standalone crate with five modules:

1. **`config.rs`** — TOML config parsing with CLI override merging.
   Config sections: Server (bind_address, max_connections, timeout_secs),
   Database (persistence_dir, wal_mode, checkpoint_interval_secs),
   Topology (auto_load, file_path), Lease (max_active, default_ttl_secs,
   fairness_policy), Logging (level, format, output).

2. **`coordinator.rs`** — Core state manager. `Mutex<CoordinatorState>`
   holds topology, leases, plans, config. Methods: `load_topology()`,
   `insert_lease()`, `insert_plan()`, `mark_node_failed()`, `flush()`.
   Graceful shutdown via `AtomicBool` flag + ctrlc handler. Dirty-state
   detection flushes to SQLite before exit.

3. **`wire_server.rs`** — TCP wire transport server per spec 025.
   Handles heartbeat, health_check, probe_request message types.
   Per-connection timeout, connection limiting, `try_clone()` for
   concurrent read/write on TCP streams.

4. **`health.rs`** — JSON health response: status (healthy/degraded/starting),
   uptime_secs, topology_epoch, active_leases, active_plans.

5. **`main.rs`** — clap CLI with start/health/status subcommands.
   ctrlc handler sets shutdown flag. start loads config + topology +
   leases from SQLite, spawns wire server, blocks until shutdown.

## Alternatives considered

### A. Async runtime (tokio) for wire server
Rejected: the daemon is I/O-bound but connection count is low (single-digit
nodes in R3). `std::net::TcpListener` with `try_clone()` is simpler and
avoids tokio version conflicts with the rest of the workspace. If R3+
demands hundreds of concurrent connections, migrate to tokio then.

### B. gRPC for wire transport
Rejected: spec 025 defines a JSON-over-TCP wire format. gRPC would
require protobuf schema generation and HTTP/2 framing, adding complexity
without consumer demand. JSON is already the interop format between
Go (`cmd/wire/`) and Rust (`fabric-graph`).

### C. In-process SQLite (rusqlite)
Accepted: `rusqlite` with WAL mode provides single-process durability
without external database dependencies. The daemon is the sole SQLite
writer — no locking contention. Checkpoint interval is configurable.

## Consequences

- `fabric-daemon` is the canonical daemon entry point.
- Config is TOML (human-friendly, diffable) with CLI overrides (operator
  convenience).
- State is flushed to SQLite on graceful shutdown and on configurable
  interval.
- Wire server handles spec 025 message types without needing the full
  Go wire package — JSON round-trip is sufficient.
- 15 unit tests. All pass.

## References

- Spec 025 (wire transport contract) — message types and wire format.
- ADR-0026 (workspace persistence format) — JSONL → SQLite migration
  path for durable state.
- ADR-0030 (route failover model) — daemon handles failover at the
  topology level (mark_node_failed → notify leases).
- ADR-0032 (multi-hop route compiler) — daemon calls `compile_multihop()`
  for multi-hop plan generation.
