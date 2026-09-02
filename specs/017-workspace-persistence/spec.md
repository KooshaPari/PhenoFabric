# 017 — Workspace persistence and seat-lease lifecycle

**Status:** proposed
**Release gate:** R1
**Work package:** PF-WP-022
**Author:** Fabric session 2026-09-01
**Depends on:** PF-WP-020 (route compiler), PF-WP-021 (route failover)
**Implements:** PF-FR-031, PF-FR-032, PF-FR-033
**ADRs:** [0025](../../adr/0025-route-lease-semantic-model.md)

## Intent

Provide a persistent, on-disk workspace store that owns the seat-lease lifecycle for the entire route lifecycle: acquire, release, expire, and revoke. The store is the single source of truth for "which seats are currently held by which workspaces" across the local machine.

The route compiler produces a `RoutePlan` (PF-WP-020); the workspace store turns that plan into a `Workspace` with one or more `SeatLease`s. Leases have a TTL, an optional trust scope, and a state machine (`Pending → Active → Released/Revoked/Expired`).

## Scope (R1)

1. **Workspace journal** — JSON-on-disk file at `~/.config/fabric/workspaces/<id>/workspace.json` (Linux/macOS) with atomic write semantics.
2. **Seat-lease state machine** — 5-state FSM: `Pending`, `Active`, `Released`, `Revoked`, `Expired` with `Display` + `Debug` + `Serialize`/`Deserialize`.
3. **Conflict detection** — when a workspace is acquired, check no other workspace holds a seat at the same `LocalityTier` for the same `NodeId`.
4. **TTL expiry** — every load of the store sweeps leases where `expires_at < now()` and transitions them to `Expired`.
5. **CLI surface** — `fabric workspace {list,show,acquire,release,gc}` as a thin wrapper around the store API.
6. **Trust scope** — initially `Ephemeral` only (local-only); `Persistent` (cross-host) deferred to R2.

## Non-goals (deferred)

- Cross-host lease replication (R2 — requires a shared store backend like Tracera).
- Lease-aware route re-compilation (R1 leases are write-only from the route compiler's perspective).
- Surface-plane integration with the lease store (PF-WP-015).

## Data model

```text
Workspace
  id: WorkspaceId (uuid v7)
  intent_id: IntentId
  topology_hash: BLAKE3 hash of the Topology
  created_at: DateTime
  expires_at: Option<DateTime>
  state: LifecycleState
  leases: Vec<SeatLease>
  trust_scope: TrustScope
  plan: Option<RoutePlan>
  notes: Option<String>

SeatLease
  seat_id: SeatId
  locality_tier: LocalityTier
  node_id: NodeId
  state: LeaseState
  acquired_at: DateTime
  expires_at: DateTime
  revoked_reason: Option<String>
  holder: WorkspaceId
```

## File format

`workspace.json` is a JSON document conforming to ADR-0026. All fields are stable schema; new optional fields may be added in minor versions; breaking changes require a new `schema_version`.

## State transitions

```
Pending  --acquire-->  Active
Active   --release-->  Released
Active   --expire-->   Expired  (TTL sweep)
Active   --revoke-->   Revoked
```

All transitions are valid only from the source state. Invalid transitions return `Error::InvalidState { state: LifecycleState }`.

## CLI commands

| Command | Purpose |
|:--|:--|
| `fabric workspace list` | List all active and pending workspaces |
| `fabric workspace show <id>` | Print workspace + leases as JSON |
| `fabric workspace acquire --intent <id>` | Create workspace from a compiled intent |
| `fabric workspace release <id>` | Mark all leases as Released |
| `fabric workspace gc` | Sweep expired leases from disk |

## Definition of done

- [ ] `fabric-workspace` crate compiles and is in the workspace
- [ ] All 5 CLI commands are wired into `fabric-cli` and tested
- [ ] Workspace journal round-trips through serde without loss
- [ ] TTL sweep runs on every load (no separate background thread in R1)
- [ ] Conflict detection rejects double-acquisition of the same `(seat, locality)`
- [ ] Integration test: `fabric cap probe → fabric graph build → fabric route plan → fabric workspace acquire → fabric workspace release` works end-to-end on a single host
