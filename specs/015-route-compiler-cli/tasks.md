# Tasks 015 — Route Compiler and Reference CLI

| ID | Task | Phase | Definition of done |
|:--|:--|:--|:--|
| PF-WP-020.01 | Implement `Node` / `Port` / `Link` / `Domain` types in `fabric-graph::model` | P0 | `cargo test -p fabric-graph` passes; `Node::roundtrip` test verifies JSON serialization matches `route.schema.json` |
| PF-WP-020.02 | Implement `CapabilityNegotiator` + `Format::is_compatible_with` + `NegotiatedPath` | P1 | `cargo test -p fabric-graph negotiation` passes; `negotiate` returns path with zero conversions when formats match |
| PF-WP-020.03 | Implement `prepare` / `commit` / `abort` / `rollback` lifecycle | P2 | `cargo test -p fabric-runtime lifecycle` passes; same input + same `fencing_token` produces identical `OperationRecord` |
| PF-WP-020.04 | Implement `Lease` + `acquire` / `release` / `revoke` + `fencing_token` | P3 | `cargo test -p fabric-runtime lease` passes; stale token returns `Err(StaleToken)` and does not allocate resource |
| PF-WP-020.05 | Implement `Workspace` + `snapshot` / `diff` / `restore` | P4 | `cargo test -p fabric-runtime workspace` passes; two identical snapshots produce the same `blake3` ID |
| PF-WP-020.06 | Implement route compiler + recursion/hop prevention | P5 | `cargo test -p fabric-graph compile` passes; `A→B→A` rejected with `RecursionDetected`; 17-stage route rejected with `HopLimitExceeded` |
| PF-WP-020.07 | Implement `fabric` CLI + JSON-RPC over unix socket | P6, P7 | `cargo test -p fabric-cli` passes; `fabric route compile --input -` roundtrips JSON; `fabric <subcommand> --help` stable; `curl --unix-socket` produces the documented response |

## Cross-deps

- Depends on: `014-capability-inventory` (R0) — `fabric-capability` provides the `CapabilityDescriptor` used by negotiation
- Blocks: `006-seamless-surface-presentation` (R2) — surfaces need a compiled route to attach to
- Blocks: `007-platform-continuity-adapters` (R3) — adapters receive a route lease and bind to it
