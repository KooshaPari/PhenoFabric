# Spec 015 — Route Compiler and Reference CLI

**Release gate:** R1 (depends on R0 capability inventory).
**Domain:** Physical-execution substrate.
**Owner:** Fabric runtime team.

---

## 1. Purpose

Define and ship the **canonical graph runtime** — node/port/link/domain objects, capability/format negotiation, prepare/commit/abort/rollback semantics, lease/fencing primitives, workspace snapshots, recursion prevention — and the **reference CLI** (`fabric`) that exposes these operations to humans and the ecosystem.

This is the second of two R0.5/R1 deliverables. R0 (capability inventory) tells you *what is where*; this spec tells you *how to move data between those things* with explicit cost, leases, and fencing.

---

## 2. Glossary

| Term | Definition |
|:--|:--|
| **Node** | An actor that produces or consumes data (e.g., an `input.udev` node, a `display.x11` node, an `app.window` node). |
| **Port** | A typed endpoint on a node. Each port declares a `format` and a `direction` (`source` or `sink`). |
| **Link** | A typed connection between two ports. Links are annotated with `locality_tier` (L0..L8) and a `transport_cost` (bytes/sec + latency_us). |
| **Domain** | A namespace for node ids. Domains form the basis of non-stealing semantics: a node in domain `phenotype://host/kosha-ws1` is owned by that host and cannot be exported by another. |
| **Capability** | A typed contract a port requires or offers (e.g., `video.h264.8bit.yuv420p`, `audio.pcm.s16le.48k.stereo`). |
| **Lease** | A time-bounded grant to occupy a node, link, or port. Leases carry a `fencing_token` and a `deadline`. |
| **Fencing token** | A monotonically-increasing counter. Resource managers must reject any request whose fencing token is older than the current token. This is the correctness primitive that prevents split-brain. |
| **Topology epoch** | A monotonic counter on the underlying capability inventory. Every time the inventory changes (capability added/removed/moved), the epoch increments. Routes captured against a given epoch become invalid on epoch change. |
| **Workspace** | A named, versioned bundle of leases + their resource graphs. Workspaces are the user-visible unit ("my Ableton setup", "agent-y setup 3"). |
| **Route** | A compiled plan from source port to sink port, composed of one or more `stages` (capture, copy, transform, encode, transport, etc.). |
| **Stage** | A single hop in a route. Each stage has an `adapter` and declares its `inputFormat` / `outputFormat`. |
| **Recursion** | A route that, when interpreted, would re-invoke the same port. Detected by static cycle search + per-lease admission control. |

---

## 3. Architecture

```
                 ┌─────────────────────────────────────────────┐
                 │  fabric CLI  (user + agent)                  │
                 │   └─> operations: cap, route, workspace,    │
                 │              topology, sign, verify          │
                 └──────────────────┬──────────────────────────┘
                                    │ JSON-RPC / native
                 ┌──────────────────▼──────────────────────────┐
                 │  fabric-runtime (graph + lease manager)      │
                 │                                              │
                 │   ┌──────────────┐    ┌───────────────────┐   │
                 │   │  Graph       │    │  Lease manager    │   │
                 │   │  compiler    │◄──►│  (fencing tokens) │   │
                 │   └──────────────┘    └───────────────────┘   │
                 │                                              │
                 │   ┌──────────────┐    ┌───────────────────┐   │
                 │   │  Capability  │    │  Workspace store  │   │
                 │   │  negotiator  │    │  (snapshots/diff) │   │
                 │   └──────────────┘    └───────────────────┘   │
                 └──────────────────┬──────────────────────────┘
                                    │ (capability descriptors)
                 ┌──────────────────▼──────────────────────────┐
                 │  fabric-capability (R0 — already shipped)    │
                 └─────────────────────────────────────────────┘
```

The CLI is the **user-facing surface** and the **agent-facing contract**. Every other ecosystem product (thegent, ShareCLI, AGSLAG) calls Fabric through this CLI's JSON-RPC or library form.

---

## 4. Sub-tasks

### 4.1 PF-WP-020.01 — Canonical node/port/link/domain objects

Define Rust types in `crates/fabric-graph::model` matching the JSON schemas in `architecture/schemas/`:

- `Node` (id, domain, kind, attributes)
- `Port` (id, node_id, name, direction, format, required_capabilities)
- `Link` (id, source_port, sink_port, locality_tier, transport_cost)
- `Domain` (id, owner, trust_boundary)

All types derive `JsonSchema`, `Serialize`, `Deserialize`, `PartialEq`, `Eq`, `Hash`. The `id` fields use the identifier scheme defined in `program/identifiers.md` (URI form `pf://host/<host>/node/<id>`).

### 4.2 PF-WP-020.02 — Capability and format negotiation

- `CapabilityNegotiator` trait: `negotiate(offered: &[Capability], required: &[Capability]) -> Result<NegotiatedPath, NegotiationError>`
- `Format` is a structured enum (mime, params, codec) with explicit `is_compatible_with(&self, &Format) -> bool`
- `NegotiatedPath` is the chain of format conversions needed (e.g., `h264.8bit -> vaapi.bitstream -> null`)
- Used by the graph compiler to validate that any candidate route is realizable on the current host set

### 4.3 PF-WP-020.03 — Graph prepare/commit/abort/rollback

- `prepare()`: produce a `GraphDraft` containing all leases, topology checks, capability matchings, format negotiations
- `commit()`: atomically transition the draft to `active` — all leases become valid
- `abort()`: destroy the draft without applying
- `rollback()`: apply an inverse plan (requires a `Workspace` snapshot)
- All operations are idempotent (same `fencing_token` input → same result)
- All operations write an `OperationRecord` to the audit log

### 4.4 PF-WP-020.04 — Lease/fencing primitives

- `Lease { id, node_id, port_id, fencing_token, deadline, holder, revocable }`
- `acquire(request) -> Result<Lease, LeaseError>`: contact resource manager, get back a token
- `release(lease) -> Result<(), LeaseError>`: voluntary release
- `revoke(lease) -> Result<(), LeaseError>`: force release (e.g., on topology change)
- `fencing_token` is `u64`, monotonically increasing per resource, persisted in the resource manager
- `is_valid(lease) -> bool` is the **only** check resource managers do — strictly greater than current token

### 4.5 PF-WP-020.05 — Workspace snapshots and diff

- `Workspace { id, name, created, leases, graph_snapshot, fencing_epoch }`
- `snapshot() -> Workspace`: capture current state (leases + topology)
- `diff(other: &Workspace) -> WorkspaceDiff`: show additions, removals, modifications
- `restore(snapshot: &Workspace) -> Result<(), WorkspaceError>`: roll back to a known-good state
- Snapshots are content-addressed (`blake3(canonical_bytes)`)

### 4.6 PF-WP-020.06 — Route recursion/hop prevention

- Static check: graph compiler rejects any graph that contains a cycle where one stage's output is fed back to one of its inputs
- Dynamic check: lease manager rejects any request whose holder would create a cycle in the current live graph
- Hop limit: any route with more than 16 stages is rejected (configurable; default 16)
- Per-port admission control: a port can be in at most one active route by default; override requires `multi-tenant: true` on the port

### 4.7 PF-WP-020.07 — CLI/SDK/API graph operations

- `fabric cap ls / show / verify / sign` — surface for `fabric-capability` (R0)
- `fabric graph ls / show / validate / diff` — surface for the inventory
- `fabric route propose / compile / commit / abort / rollback / show` — full route lifecycle
- `fabric workspace list / create / snapshot / restore / diff / show` — workspace management
- `fabric topology epoch / wait` — topology change observation
- All commands accept JSON input (`--input -` or `--input file.json`) and produce JSON output (`--output -` or `--output file.json`)
- Stable exit codes: `0` success, `1` user error, `2` resource error, `3` fencing failure, `4` topology invalid, `5` negotiation failure
- JSON-RPC interface on a unix socket (default: `~/.fabric/socket`) for agent use

---

## 5. Non-goals (this spec)

- Implementing actual transport (NVMS, RDMA, wayland, etc.) — the CLI is the *control plane*, not the *data plane*
- Persistent multi-host clustering (single-host only for R1)
- UI/GUI

---

## 6. Acceptance criteria

- All seven sub-tasks implemented in the `fabric-graph`, `fabric-runtime`, and `fabric-cli` crates
- ≥ 30 Rust tests passing
- `fabric --help` and `fabric <subcommand> --help` produce stable, machine-readable output
- All commands roundtrip through JSON (input and output both parse)
- The route compiler rejects the canonical recursion example (A→B→A) with exit code 4
- The lease manager rejects a stale fencing token with exit code 3
- The CLI is shipped as a single static binary (`cargo build --release -p fabric-cli`)
