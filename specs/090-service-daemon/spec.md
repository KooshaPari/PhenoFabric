# Spec 090 — Service-Mode Daemon (PF-WP-090, R3)

**Status:** draft
**WP:** PF-WP-090
**Release gate:** R3
**Depends on:** specs/080 (persistent state), specs/024 (surface runtime), specs/025 (wire transport)
**Enables:** fleet-ops integration, long-running coordinator

## 1. Background

R0-R2 run as one-shot CLI tools: `fabric-graph-cli replan`, `cmd/checker`, `cmd/trust`. Each invocation starts fresh, does its work, and exits.

R3 introduces a long-running daemon that:
- holds topology in memory (survives across requests);
- manages leases with TTL and expiry;
- accepts wire-transport connections from endpoints;
- persists state to SQLite (spec 080);
- exposes a control API for operators.

## 2. Scope

**In:**
- `fabric-daemon` crate: long-running coordinator process
- Signal handling (SIGTERM/SIGINT graceful shutdown)
- SQLite persistence integration (spec 080)
- Wire transport server (spec 025)
- Health check endpoint
- Structured logging (tracing crate)
- Configuration via CLI args + config file + env vars

**Out:**
- Multi-node consensus (R4)
- gRPC API (R3.1)
- Web dashboard (R3.1)
- mTLS (R3.1, spec 025 defers to R3)

## 3. Architecture

```
fabric-daemon
├── main.rs           // entry point, signal handling, config loading
├── config.rs         // Configuration (CLI + file + env)
├── coordinator.rs    // Core coordinator logic
├── wire_server.rs    // Wire transport listener (spec 025)
├── health.rs         // Health check endpoint
└── logging.rs        // Structured logging setup
```

## 4. CLI Interface

```bash
# Start the daemon
fabric-daemon start \
  --config /etc/phenotype/daemon.toml \
  --db /var/lib/phenotype/state.db \
  --listen 127.0.0.1:9400 \
  --log-level info

# Check health
fabric-daemon health --connect 127.0.0.1:9400

# Status
fabric-daemon status --connect 127.0.0.1:9400

# Stop
fabric-daemon stop --connect 127.0.0.1:9400
```

## 5. Configuration

```toml
# /etc/phenotype/daemon.toml
[server]
listen = "127.0.0.1:9400"
max_connections = 64
request_timeout_ms = 5000

[database]
path = "/var/lib/phenotype/state.db"
wal_mode = true
flush_interval_ms = 1000

[topology]
auto_probe = true
probe_interval_s = 30
epoch_persistence = true

[leases]
default_ttl_s = 3600
max_ttl_s = 86400
renewal_window_s = 300
fairness_policy = "FairShare"

[logging]
level = "info"
format = "json"
file = "/var/log/phenotype/daemon.log"
```

## 6. Lifecycle

```
1. Parse config (CLI > file > env > defaults)
2. Open SQLite (spec 080)
3. Run migrations
4. Recover state from DB
5. Start wire transport server
6. Start health check endpoint
7. Begin accepting connections
8. On SIGTERM/SIGINT:
   a. Stop accepting new connections
   b. Wait for in-flight requests (5s timeout)
   c. Flush dirty state to SQLite
   d. Close SQLite
   e. Exit 0
```

## 7. Wire Transport Server

Listens on TCP/Unix socket per spec 025. Accepts `WireEnvelope` messages:
- `ProbeRequest` → respond with capability descriptor
- `ReplanRequest` → run failover::replan, return response
- `SurfaceInvalidate` → process invalidation, update leases
- `Heartbeat` → respond with heartbeat ack

## 8. Health Check

```json
{
  "status": "healthy",
  "uptime_s": 3600,
  "topology_epoch": 42,
  "active_leases": 7,
  "active_plans": 3,
  "db_size_bytes": 1048576
}
```

## 9. Graceful Shutdown

```
Received SIGTERM
  → set shutdown_flag = true
  → reject new connections with "shutting_down"
  → wait for in-flight (max 5s)
  → flush dirty state
  → close SQLite
  → exit 0
```

## 10. Dependencies

- `fabric-graph` (in-memory graph)
- `fabric-persist` (SQLite)
- `fabric-graph-cli` (replan protocol)
- `tokio` (async runtime)
- `tracing` / `tracing-subscriber` (logging)
- `clap` (CLI args)
- `toml` (config file)
- `rusqlite` (via fabric-persist)

## 11. File Locations

| File | Purpose |
|------|---------|
| `crates/fabric-daemon/src/main.rs` | Entry point |
| `crates/fabric-daemon/src/config.rs` | Configuration |
| `crates/fabric-daemon/src/coordinator.rs` | Core logic |
| `crates/fabric-daemon/src/wire_server.rs` | Wire transport |
| `crates/fabric-daemon/src/health.rs` | Health check |
| `crates/fabric-daemon/src/logging.rs` | Logging setup |
| `crates/fabric-daemon/tests/` | Integration tests |