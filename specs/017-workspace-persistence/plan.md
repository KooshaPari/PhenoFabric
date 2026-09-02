# 017 — Workspace persistence plan

## P0 (this session)

- [x] Spec definition
- [x] ADR-0026 file format
- [x] Source files preserved untracked (Cargo.toml, src/lib.rs, src/lease.rs, src/state.rs, src/error.rs)

## P1 (next session)

- [ ] Add `fabric-workspace` to workspace Cargo.toml
- [ ] Make `Workspace` and `SeatLease` derive `Serialize, Deserialize` correctly
- [ ] Implement `WorkspaceStore::load` and `WorkspaceStore::save` with atomic write (write to .tmp, rename)
- [ ] Implement `WorkspaceStore::acquire` with conflict detection
- [ ] Implement TTL sweep on every `load`
- [ ] Wire `fabric-workspace` into `fabric-cli` (5 sub-commands: list/show/acquire/release/gc)
- [ ] Add integration test that runs the full pipeline on a single host

## P2 (R1+)

- [ ] Persistent trust scope (cross-host leases via Tracera shared store)
- [ ] Background sweep thread
- [ ] Lease-aware route re-compilation hooks

## Acceptance criteria

1. `cargo test -p fabric-workspace` passes
2. `fabric workspace list` shows currently active leases
3. Two concurrent `fabric workspace acquire` calls on the same `(seat, locality)` produce exactly one Active and one `Error::Conflict`
4. A workspace with `expires_at < now()` is swept to Expired on the next `load`
5. `workspace.json` matches the schema in `architecture/schemas/workspace.schema.json` (created by this spec)
