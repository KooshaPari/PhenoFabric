# Plan 015 — Route Compiler and Reference CLI

## Phases

| Phase | Scope | Exit criterion |
|:--|:--|:--|
| **P0** | Types: `Node`, `Port`, `Link`, `Domain`, `Capability`, `Format`, `Lease` in `fabric-graph::model` | `cargo test -p fabric-graph` passes; `jsonschema` validates the round-trip |
| **P1** | Negotiation: `CapabilityNegotiator`, `Format::is_compatible_with`, `NegotiatedPath` | `cargo test -p fabric-graph negotiation` passes |
| **P2** | Lifecycle: `prepare` / `commit` / `abort` / `rollback` with `OperationRecord` audit log | All four operations idempotent; `cargo test -p fabric-runtime lifecycle` passes |
| **P3** | Lease manager: `acquire` / `release` / `revoke` / `fencing_token` monotonic, `is_valid` strictly-greater check | `cargo test -p fabric-runtime lease` passes; stale-token test returns `Err(StaleToken)` |
| **P4** | Workspace: `snapshot` / `diff` / `restore` with content-addressed IDs | `cargo test -p fabric-runtime workspace` passes; identical snapshots produce same `blake3` ID |
| **P5** | Route compiler: recursion/hop prevention, 16-stage limit, per-port admission | `cargo test -p fabric-graph compile` passes; A→B→A is rejected with `RecursionDetected` |
| **P6** | CLI: `fabric` binary with `cap`, `graph`, `route`, `workspace`, `topology` subcommands | `cargo test -p fabric-cli` passes; `--help` stable; JSON roundtrip on each subcommand |
| **P7** | JSON-RPC surface on unix socket | `cargo test -p fabric-cli rpc` passes; round-trip via `curl --unix-socket` |

## WP DAG

```
PF-WP-020.01 ──┐
               ├──> PF-WP-020.02 ──> PF-WP-020.03 ──> PF-WP-020.07
PF-WP-014.* ───┘                              │
                                              ├──> PF-WP-020.04
                                              ├──> PF-WP-020.05
                                              └──> PF-WP-020.06
```

## Acceptance

The spec is "complete" when:
1. All 7 sub-tasks are merged
2. The release-gate review in `releases/` is filed
3. CI is green (`spec-validation` + `cargo test --workspace` + `go test ./...`)
4. The CLI binary builds to ≤ 10 MB (release profile, stripped)
