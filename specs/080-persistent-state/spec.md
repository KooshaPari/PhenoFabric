# Spec 080 — Persistent State (PF-WP-080, R3)

**Status:** draft
**WP:** PF-WP-080
**Release gate:** R3
**Depends on:** specs/020 (route lease integration), specs/022 (multi-tenant fairness), ADR-0026 (workspace persistence format)
**Enables:** PF-WP-090 (service daemon), PF-WP-070 (multi-hop compiler state)

## 1. Background

R0-R2 operate entirely in memory. Topology, leases, route plans, and evidence
exist only for the lifetime of the process. A coordinator crash loses all state.

R3 introduces persistence so that:
- topology survives restarts (probes don't need to re-run);
- active leases survive coordinator restarts (endpoints don't lose authority);
- audit/evidence records are durable (operator can investigate later);
- workspace snapshots are recoverable (user doesn't re-apply patches).

ADR-0026 already defines the JSONL workspace persistence format. This spec
extends that to a full SQLite-backed persistence layer for the coordinator's
internal state.

## 2. Scope

**In:**

- `fabric_persist` crate: SQLite persistence for topology, leases, route plans, evidence
- Schema migrations (append-only, forward-only)
- Atomic write transactions (WAL mode)
- JSONL workspace state (per ADR-0026)
- Append-only audit/evidence log
- Startup recovery: reload topology + leases from DB
- Graceful shutdown: flush in-flight state to DB

**Out:**

- Distributed persistence (Raft replication) — R4
- Time-series metrics storage — separate system
- Backup/restore CLI — R3.1
- Encryption at rest — R3.1 (SQLitecipher)

## 3. Architecture

```
┌─────────────────────────────────────────────────┐
│              fabric-daemon (R3)                  │
│  ┌──────────────┐  ┌──────────────────────────┐ │
│  │ fabric-graph  │  │ fabric-persist            │ │
│  │ (in-memory)   │  │ (SQLite + JSONL)          │ │
│  │               │  │                           │ │
│  │ Topology      │◄─│ topology table            │ │
│  │ Leases        │◄─│ leases table              │ │
│  │ RoutePlans    │◄─│ route_plans table         │ │
│  │ SurfaceRt     │◄─│ surface_bindings table    │ │
│  │ Fairness      │   │ (derived from leases)     │ │
│  │               │  │                           │ │
│  │               │──│ evidence log (append)     │ │
│  │               │──│ audit log (append)        │ │
│  └──────────────┘  └──────────────────────────┘ │
│         │                    │                   │
│         ▼                    ▼                   │
│  ┌──────────────┐  ┌──────────────────────────┐ │
│  │ In-memory    │  │ SQLite WAL                │ │
│  │ (hot path)   │  │ (durable)                 │ │
│  └──────────────┘  └──────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

The in-memory graph is the source of truth during operation. SQLite is the
durability layer. On startup, the daemon loads from SQLite into memory.
During operation, mutations are written to SQLite asynchronously (WAL mode
allows concurrent reads).

## 4. Schema

### 4.1 topology_nodes

```sql
CREATE TABLE topology_nodes (
    id TEXT PRIMARY KEY,
    label TEXT,
    locality_tier INTEGER NOT NULL,
    tags TEXT, -- JSON array
    capability_refs TEXT, -- JSON array of CapabilityRef
    trust_level INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL, -- RFC3339
    updated_at TEXT NOT NULL, -- RFC3339
    deleted_at TEXT -- soft delete
);
```

### 4.2 topology_edges

```sql
CREATE TABLE topology_edges (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES topology_nodes(id),
    target_id TEXT NOT NULL REFERENCES topology_nodes(id),
    locality_tier INTEGER NOT NULL,
    link_metrics TEXT, -- JSON LinkMetrics
    created_at TEXT NOT NULL,
    deleted_at TEXT
);
```

### 4.3 topology_meta

```sql
CREATE TABLE topology_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
-- Stores: name, epoch (INTEGER), description
```

### 4.4 leases

```sql
CREATE TABLE leases (
    id TEXT PRIMARY KEY,
    surface_handle TEXT NOT NULL,
    spec TEXT NOT NULL, -- JSON SurfaceSpec
    state TEXT NOT NULL, -- Pending|Active|Completed|Failed|Revoked|Expired
    binding_id TEXT, -- UUID of active binding
    binding_node TEXT, -- NodeId of current binding
    tenant_id TEXT, -- for fairness queue
    fairness_weight INTEGER DEFAULT 1,
    epoch INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    expires_at TEXT,
    revoked_at TEXT,
    revocation_reason TEXT
);
CREATE INDEX idx_leases_state ON leases(state);
CREATE INDEX idx_leases_binding_node ON leases(binding_node);
CREATE INDEX idx_leases_tenant ON leases(tenant_id);
```

### 4.5 route_plans

```sql
CREATE TABLE route_plans (
    id TEXT PRIMARY KEY,
    intent_id TEXT NOT NULL,
    topology_epoch INTEGER NOT NULL,
    steps TEXT NOT NULL, -- JSON Vec<RouteStep>
    estimated_latency_us REAL,
    score TEXT, -- JSON ScoreBreakdown
    compiled_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    tags TEXT, -- JSON array
    state TEXT NOT NULL DEFAULT 'active', -- active|expired|replaced
    replaced_by TEXT, -- FK to route_plans.id
    created_at TEXT NOT NULL
);
CREATE INDEX idx_route_plans_intent ON route_plans(intent_id);
CREATE INDEX idx_route_plans_state ON route_plans(state);
```

### 4.6 evidence_log

```sql
CREATE TABLE evidence_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    event_id TEXT NOT NULL UNIQUE,
    producer_id TEXT,
    principal_id TEXT,
    correlation_id TEXT,
    topology_epoch INTEGER,
    payload TEXT NOT NULL, -- JSON
    signature TEXT, -- base64 Ed25519
    observed_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_evidence_type ON evidence_log(event_type);
CREATE INDEX idx_evidence_observed ON evidence_log(observed_at);
```

### 4.7 audit_log

```sql
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action TEXT NOT NULL,
    actor TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    details TEXT, -- JSON
    result TEXT NOT NULL, -- success|failure
    observed_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_audit_action ON audit_log(action);
CREATE INDEX idx_audit_resource ON audit_log(resource_type, resource_id);
```

### 4.8 schema_version

```sql
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL,
    description TEXT
);
```

## 5. Operations

### 5.1 Save topology

```rust
impl Persist {
    pub fn save_topology(&self, topology: &Topology) -> Result<(), PersistError>;
    pub fn load_topology(&self) -> Result<Topology, PersistError>;
    pub fn save_node(&self, node: &Node) -> Result<(), PersistError>;
    pub fn delete_node(&self, id: &NodeId) -> Result<(), PersistError>;
    pub fn save_edge(&self, edge: &Edge) -> Result<(), PersistError>;
    pub fn delete_edge(&self, id: &EdgeId) -> Result<(), PersistError>;
    pub fn advance_epoch(&self) -> Result<TopologyEpoch, PersistError>;
}
```

### 5.2 Save leases

```rust
impl Persist {
    pub fn save_lease(&self, lease: &SurfaceLease) -> Result<(), PersistError>;
    pub fn load_active_leases(&self) -> Result<Vec<SurfaceLease>, PersistError>;
    pub fn load_leases_by_node(&self, node: &NodeId) -> Result<Vec<SurfaceLease>, PersistError>;
    pub fn expire_lease(&self, id: &Uuid) -> Result<(), PersistError>;
    pub fn revoke_lease(&self, id: &Uuid, reason: &str) -> Result<(), PersistError>;
}
```

### 5.3 Save route plans

```rust
impl Persist {
    pub fn save_route_plan(&self, plan: &RoutePlan) -> Result<(), PersistError>;
    pub fn load_active_plans(&self) -> Result<Vec<RoutePlan>, PersistError>;
    pub fn expire_plan(&self, id: &RoutePlanId) -> Result<(), PersistError>;
    pub fn replace_plan(&self, old: &RoutePlanId, new: &RoutePlan) -> Result<(), PersistError>;
}
```

### 5.4 Evidence and audit

```rust
impl Persist {
    pub fn append_evidence(&self, event: &EventEnvelope) -> Result<(), PersistError>;
    pub fn query_evidence(
        &self,
        event_type: Option<&str>,
        since: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<EventEnvelope>, PersistError>;

    pub fn append_audit(&self, entry: &AuditEntry) -> Result<(), PersistError>;
    pub fn query_audit(
        &self,
        action: Option<&str>,
        resource: Option<(&str, &str)>,
        since: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<AuditEntry>, PersistError>;
}
```

### 5.5 Startup recovery

```rust
impl Persist {
    /// Load all active state from SQLite into an in-memory graph.
    /// Called once at daemon startup.
    pub fn recover_state(&self) -> Result<RecoveredState, PersistError>;
}

pub struct RecoveredState {
    pub topology: Topology,
    pub active_leases: Vec<SurfaceLease>,
    pub active_plans: Vec<RoutePlan>,
    pub last_evidence_id: i64,
    pub last_audit_id: i64,
}
```

## 6. Write Path

```
Mutation (in-memory)
  │
  ├─► Write to SQLite (WAL, async)
  │     └─ On success: mark as persisted
  │     └─ On failure: log error, retry on next flush
  │
  └─► Return to caller (in-memory is authoritative)
```

The in-memory graph is always the source of truth during operation.
SQLite writes are async and best-effort during normal operation.
On graceful shutdown, the daemon flushes all dirty state synchronously.

## 7. Startup Recovery

```
1. Open SQLite (WAL mode)
2. Run migrations if needed
3. Load topology_nodes + topology_edges + topology_meta
4. Load active leases (state IN ('Pending', 'Active'))
5. Load active route_plans (state = 'active')
6. Load last evidence/audit IDs (for sequence continuity)
7. Construct in-memory graph from loaded state
8. Begin accepting requests
```

If the DB is empty (first run), start with an empty topology.

## 8. Graceful Shutdown

```
1. Stop accepting new requests
2. Wait for in-flight requests to complete (timeout: 5s)
3. Flush all dirty in-memory state to SQLite
4. Close SQLite connection
5. Exit
```

## 9. Workspace JSONL (per ADR-0026)

The workspace state file at `${XDG_STATE_HOME}/phenotype/fabric/workspace.jsonl`
remains separate from SQLite. It is the user-facing persistence format.

The daemon reads/writes both:
- SQLite: internal coordinator state (topology, leases, routes, evidence)
- JSONL: user-facing workspace state (desired graph, patches, snapshots)

## 10. Migration Strategy

- Schema version is tracked in `schema_version` table
- Migrations are append-only (never modify existing columns)
- Forward-only: no down migrations
- Each migration is a numbered SQL file applied in order

## 11. Error Handling

```rust
#[derive(Error, Debug)]
pub enum PersistError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("migration error: {0}")]
    Migration(String),

    #[error("recovery error: {0}")]
    Recovery(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

## 12. Performance Targets

- Topology save: <10ms for 1000 nodes
- Lease save: <5ms per lease
- Startup recovery: <100ms for 1000 nodes + 100 leases
- Evidence append: <1ms per event
- Audit query: <50ms for 1000 records

SQLite WAL mode provides concurrent reads during writes.
The daemon does not block on persistence during normal operation.

## 13. Dependencies

- `rusqlite` (SQLite bindings, feature = "bundled")
- `serde` / `serde_json` (serialization)
- `uuid` (lease/binding IDs)
- `chrono` (timestamps)
- `thiserror` (error types)

## 14. File Locations

| File | Purpose |
|------|---------|
| `crates/fabric-persist/src/lib.rs` | Public API |
| `crates/fabric-persist/src/schema.rs` | Table definitions + migrations |
| `crates/fabric-persist/src/topology.rs` | Topology CRUD |
| `crates/fabric-persist/src/leases.rs` | Lease CRUD |
| `crates/fabric-persist/src/routes.rs` | Route plan CRUD |
| `crates/fabric-persist/src/evidence.rs` | Evidence + audit log |
| `crates/fabric-persist/src/recovery.rs` | Startup recovery |
| `crates/fabric-persist/migrations/` | SQL migration files |
| `crates/fabric-persist/tests/` | Integration tests |
