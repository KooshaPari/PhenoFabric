# Spec 023 — fabric-graph-cli replan (Q1-C thin Rust binary)

**WP**: PF-WP-040 · **Release gate**: R2-first-wedge · **Status**: draft

## 1. Background

R1 closed at 100% (HEAD `724e04b`). Q1 of the four R1→R2 open questions
was: *"Where does `-checker-replan` live?"* The operator-decided answer
(Q1-C, committed in `releases/2026-09-08-R1.md` §Open questions) is:

> **C — thin Rust binary (`fabric-graph-cli replan ...`)**
> No cgo, no duplicate algorithm, isolates the failover algorithm in
> one place. The Go `cmd/checker` subprocesses the binary, reads JSON
> out, and translates back into its own `Decision` taxonomy.

This spec pins down the protocol for that binary so the R2 Go wiring
(`cmd/checker -checker-replan`) has a stable contract.

## 2. Why a separate crate from `fabric-cli`

`crates/fabric-cli/` is **explicitly Tier 3** per ADR-0028 — the 6 prior
attempts hit the codified 50-error cascade. `fabric-graph-cli` is a
**focused, single-purpose** binary that calls exactly one function:
`fabric_graph::failover::replan()`. Scope-creep is forbidden.

| Crate | Scope | Tier |
|---|---|---|
| `fabric-cli` (existing, untracked) | Full CLI: cap, graph, route, workspace | Tier 3 / fresh-context |
| `fabric-graph-cli` (this spec, NEW) | Single binary: `replan` subcommand only | R2 in-scope |

## 3. Public API

### 3.1 CLI invocation

```
fabric-graph-cli replan [--request <file>]
fabric-graph-cli replan [--topology <file>] [--intent <file>] [--old-plan <file>] [--failed-nodes <id1,id2,...>]
fabric-graph-cli --version
fabric-graph-cli --help
```

**Default mode** (no flags): read a single JSON request from stdin,
write a single JSON response to stdout.

**File mode** (--topology etc): assemble the request from individual
files. `--failed-nodes` is a comma-separated list of node IDs matching
the `cmd/checker -failover-blacklist` convention.

### 3.2 Request schema

```jsonc
{
  "topology": { /* fabric_graph::model::Topology, full */ },
  "intent":   { /* fabric_graph::model::Intent,   full */ },
  "old_plan": { /* fabric_graph::model::RoutePlan, full */ },
  "failed_nodes": ["node-id-1", "node-id-2"]
}
```

All three graph types already derive `Serialize, Deserialize` (verified
Phase 0). `failed_nodes` is a `Vec<String>` for simplicity — the binary
parses each to `NodeId`.

### 3.3 Response schema

```jsonc
{
  "outcome": "Replaced",      // one of: "Replaced" | "NoReplacement"
  "plan": { /* RoutePlan */ } // present iff outcome == "Replaced"
}
```

OR, on error:

```jsonc
{
  "error": "EmptyIntent",     // one of: "EmptyIntent" | "Io" | "Json" | "Unknown"
  "message": "intent has no requirements — nothing to replan onto"
}
```

Exit codes:
- `0` — success (Replaced or NoReplacement)
- `1` — domain error (EmptyIntent)
- `2` — I/O / parse error
- `3` — internal / unexpected

### 3.4 Scope-out (documented)

- No `compile` subcommand (out of R2 scope)
- No `negotiate` subcommand
- No `--format json|text` flag — JSON only (Go consumer wants JSON)
- No daemon mode (PF-WP-090, R3)
- No HTTP/gRPC transport (subprocess + JSON is the Q1-C choice)

## 4. Implementation

### 4.1 Crate layout

```
crates/fabric-graph-cli/
├── Cargo.toml
├── src/
│   ├── main.rs        # arg parse + stdin/stdout dispatch
│   ├── request.rs     # ReplanRequest, ReplanResponse, ReplanError
│   └── protocol.rs    # JSON <-> ReplanOutcome/FailoverOutcome mapping
└── tests/
    └── replan_e2e.rs  # builds binary, pipes JSON in, parses JSON out
```

Workspace member. Dep: `fabric-graph` (path), `serde`, `serde_json`,
`anyhow`, `thiserror`. No `clap` (hand-rolled minimal parser, matches
the dep-light posture of the rest of the workspace).

### 4.2 Error mapping (FailoverError → JSON)

| `FailoverError` | JSON `error` | Exit |
|---|---|---|
| `EmptyIntent` | `EmptyIntent` | 1 |
| `AllCandidatesFailed` | `NoReplacement` (translated to `outcome: "NoReplacement"` with `plan: null`) | 0 |

Note: `replan()` returns `AllCandidatesFailed` only if `blacklist` is
non-empty AND `compile()` fails on the pruned topology. The current
impl (verified Phase 0) translates this internally to
`FailoverOutcome::NoReplacement` without surfacing the error variant.
The spec pins this contract: callers see `NoReplacement`, not the
internal error.

### 4.3 Contract for Go consumer

The Go `cmd/checker` will:
1. Build a JSON request containing the current topology + intent + old plan + failed node IDs.
2. Subprocess `fabric-graph-cli replan` with the JSON on stdin.
3. Parse the JSON response.
4. Map `outcome == "Replaced"` → call surface-plane rebind locally.
5. Map `outcome == "NoReplacement"` → release the lease (existing path).

Stdout is reserved for the response. Logs go to stderr (already the
`tracing` crate default). The Go consumer must capture stdout separately
from stderr.

## 5. Acceptance criteria

1. `cargo build --release -p fabric-graph-cli` produces a binary.
2. `cargo test -p fabric-graph-cli` passes (>= 4 tests):
   - `replan_with_empty_blacklist_returns_old_plan`
   - `replan_after_node_pruning_produces_new_route`
   - `replan_with_no_survivors_returns_no_replacement`
   - `replan_with_empty_intent_returns_empty_intent_error`
   - `cli_end_to_end_via_stdin` (integration: spawns the binary,
     pipes JSON, asserts JSON out)
   - `cli_handles_io_error_gracefully` (integration: missing file → exit 2)
3. End-to-end smoke (operator-runnable): `cat request.json | fabric-graph-cli replan` → JSON out.
4. `check_manifest.py`, `check_json_schemas.py`, `check_openapi.py`, `check_links.py`: all green.

## 6. Out-of-scope horizons

- Wire `fabric-graph-cli replan` into `cmd/checker` Go — R2 wedge #2.
- Add `compile` / `negotiate` subcommands if needed by future R2 work.
- Replace subprocess with Unix domain socket if IPC latency becomes a
  concern (R3 candidate, deferred until measured).
- Add `fabric-workspace` integration so the response is also appended
  to the workspace event log (deferred — workspace persistence is PF-WP-017, R2 later).
