# 017 — Workspace persistence tasks

| ID | Description | Phase | Owner | Status |
|:--|:--|:--|:--|:--|
| WP-017.01 | spec + meta + plan + tasks for workspace persistence | P0 | session | DONE |
| WP-017.02 | ADR-0026 workspace file format | P0 | session | DONE |
| WP-017.03 | Cargo.toml + src/{lib,lease,state,error}.rs (preserved untracked) | P0 | session | DONE |
| WP-017.04 | Make Workspace + SeatLease derive Serialize/Deserialize correctly | P1 | next | TODO |
| WP-017.05 | Add fabric-workspace to workspace Cargo.toml | P1 | next | TODO |
| WP-017.06 | Atomic write (write to .tmp, rename) for save() | P1 | next | TODO |
| WP-017.07 | Implement WorkspaceStore::acquire with conflict detection | P1 | next | TODO |
| WP-017.08 | Implement TTL sweep on load() | P1 | next | TODO |
| WP-017.09 | Wire fabric-workspace into fabric-cli (list/show/acquire/release/gc) | P1 | next | TODO |
| WP-017.10 | Integration test: full pipeline on single host | P1 | next | TODO |
| WP-017.11 | Persistent trust scope (cross-host via Tracera) | P2 | future | TODO |
| WP-017.12 | Background sweep thread | P2 | future | TODO |
| WP-017.13 | Lease-aware route re-compilation | P2 | future | TODO |

## Definition of done (P0)

- [x] spec.md, meta.json, plan.md, tasks.md all exist
- [x] ADR-0026 written
- [x] Source files present (untracked) and reviewed for correctness
- [x] WORKLOG records the deferred completion rationale

## Definition of done (P1)

- [ ] `cargo test --workspace` passes including `fabric-workspace`
- [ ] `fabric workspace list` works on a real host
- [ ] spec-validation CI green
- [ ] No new compile warnings introduced
