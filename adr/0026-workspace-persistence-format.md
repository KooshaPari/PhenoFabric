# 0026 — Workspace Persistence File Format

## Status

Accepted — 2026-09-02.

## Date

2026-09-02.

## Deciders

GLM-backed Codex chat continuation session (2026-09-01 → 2026-09-02).

## Supersedes

None.

## Traceability

- ADR-0025 (route/lease semantic model) — defines the live data
- spec `017-workspace-persistence` — defines the contract
- `architecture/schemas/workspace.schema.json` — defines the wire format
- PRD section 6.4 (workspace state file)

## Context

`fabric-workspace` must persist active route bindings to a file the
operator can read, copy, version-control, and reload. The persistence
file is the durable surface for everything that is not currently live
in memory:

- Active seat leases and their current `LifecycleState`
- The compiled `RoutePlan` that produced each lease
- The `Topology` snapshot used to compile the plan (so a reload can
  detect drift)
- Workspace-level identity (`WorkspaceId`) and trust scope
- Last-seen epoch of the topology that bound each lease (so a reload
  can refuse leases against a newer topology than the operator's copy)
- Policy fingerprint (so reload can fail closed if the policy set
  changed)

The file is the source of truth for the **next** boot. The in-memory
state is just a cache over it. Therefore the on-disk format must be
self-describing, signed, and schema-validated.

## Decision

The on-disk file is **JSON Lines (JSONL) with a header frame**,
`schema_version` = 1, MIME `application/vnd.phenotype.fabric.workspace+jsonl`.

### File layout

```
{"header": { ... }}            # one record
{"lease": { ... }}             # zero or more records, append-only
{"lease": { ... }}
{"seat_check": { ... }}        # optional audit records
```

Header record is required, must be first, and is the only place
schema_version and the policy fingerprint live. Lease records are
the persistent form of a `SeatLease` and include the
`RoutePlan` hash they were bound from. Audit records are advisory
and not required for load.

### Path and naming

- Default: `${XDG_STATE_HOME:-${HOME}/.local/state}/phenotype/fabric/workspace.jsonl`
- Override: `--state-file <path>` on the CLI; env var
  `FABRIC_WORKSPACE_STATE` for shells
- All writes are atomic (write-temp + rename) with a `.bak` file kept
  one deep

### Required header fields

| Field | Type | Purpose |
|:--|:--|:--|
| `schema_version` | `1` | Hard-coded. Bumping requires a new ADR. |
| `workspace_id` | `pf://workspace/<uuid>` | Stable identity |
| `created_at` | RFC3339 | When the file was first created |
| `topology_epoch` | `u64` | Topology epoch the file was bound to |
| `policy_fingerprint` | `blake3:hex` | Hash of the active policy set |
| `trust_scope` | `ephemeral\|persistent` | Whether the file is per-boot or crosses boots |
| `node_id` | `pf://node/<uuid>` | Authoring host |
| `signatures` | `Vec<Signature>` | Ed25519 signatures over the header |

### Required lease record fields

| Field | Type | Purpose |
|:--|:--|:--|
| `id` | `pf://seat/<uuid>` | Lease identity (stable across state transitions) |
| `state` | `pending\|active\|released\|revoked\|expired` | Current `LifecycleState` |
| `workspace_id` | matches header | Cross-reference |
| `plan_id` | `pf://route/<uuid>` | The compiled `RoutePlan` this lease binds to |
| `plan_hash` | `blake3:hex` | Hash of the plan's canonical bytes (catches tampering) |
| `topology_epoch` | matches header | Drift detection on load |
| `created_at`, `expires_at`, `released_at?` | RFC3339 | Lifecycle timestamps |
| `signatures` | `Vec<Signature>` | At least one for `active` state |

### Atomicity and crash safety

- Writes go to `<path>.tmp`, then `rename(2)` to `<path>`
- On load, if `<path>` is missing but `<path>.tmp` exists, the loader
  retries the rename
- On load, if `<path>` is corrupt, the loader falls back to `<path>.bak`
  and emits a warning; operator decides
- Crash mid-write is a no-op on the live file (rename is atomic on
  POSIX)

### Concurrency

- Multiple writers: only one `fabric` process should hold the live
  workspace at a time. On startup, take an advisory lock
  (`flock(2)` / `fcntl(F_OFD_SETLK)`) on `<path>.lock`
- The lock is **advisory**, not mandatory. A second process that
  ignores the lock and writes will succeed, but its writes are
  appended to a separate `<path>.<pid>.append` file that the
  primary process merges on next quiet moment (not implemented in
  R0; flagged as R1 work)

### Schema validation

- `architecture/schemas/workspace.schema.json` is the wire-format spec
- `program/scripts/check_json_schemas.py` validates every JSON file
  in `architecture/schemas/` is parseable JSON Schema (a meta-check,
  not a domain check)
- Runtime validation on load: a `fabric workspace validate` subcommand
  uses the same schema to refuse malformed files before they reach
  the state machine

### Versioning policy

- `schema_version` is the version of the on-disk format
- Bumping the major version requires a new ADR and a migration
- Bumping the minor version is allowed for additive changes only
- The load code MUST refuse a file whose `schema_version` is greater
  than the runtime knows about (fail closed)

## Consequences

- Operators can read the workspace file with `jq` and grep — JSONL
  is line-oriented and human-friendly
- Reload is cheap: stream the file, validate each record against the
  schema, build an in-memory `WorkspaceStore` index
- The signing model matches the capability descriptor signing model
  (Ed25519, base64 key fingerprint, domain-separated blake3) — same
  tooling, same key, same verification path
- Drift detection: if a reload sees a `topology_epoch` older than
  the live topology, the loader must refuse the lease (fail closed)
- The file is the **source of truth for identity**, not for trust
  state — trust state is recomputed from the live capability
  descriptors at load time
- The `trust_scope=ephemeral` mode means the file is treated as
  per-boot and may be deleted safely; `persistent` mode means the
  file is the durable record and must be preserved

## Open

- The `.append` merge protocol for second-process writes is R1
- Compression (zstd) for the file is R2 — small files for R0
- Per-record signature caching is R1 — for now, every load re-verifies
