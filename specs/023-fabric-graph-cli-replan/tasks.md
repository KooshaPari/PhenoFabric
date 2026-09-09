# Tasks 023 — fabric-graph-cli replan

## Phase 0 — read source (ADR-0028)

- [ ] T023-P0-01 Read `crates/fabric-graph/src/failover.rs`
- [ ] T023-P0-02 Read `crates/fabric-graph/src/model.rs`
- [ ] T023-P0-03 Read `crates/fabric-graph/src/lib.rs`
- [ ] T023-P0-04 Read `crates/fabric-graph/src/compile.rs`
- [ ] T023-P0-05 Read workspace `Cargo.toml`
- [ ] T023-P0-06 Read `crates/fabric-capability/src/locality.rs`
- [ ] T023-P0-07 Read `cmd/checker/main.go`

## Phase 1 — scaffold

- [ ] T023-S-01 `mkdir crates/fabric-graph-cli/{src,tests}`
- [ ] T023-S-02 Author `crates/fabric-graph-cli/Cargo.toml`
- [ ] T023-S-03 Add `crates/fabric-graph-cli` to workspace members
- [ ] T023-S-04 Author `crates/fabric-graph-cli/src/lib.rs` (re-export protocol module)

## Phase 2 — protocol.rs

- [ ] T023-P-01 Define `ReplanRequest`, `ReplanResponse`, `ReplanErrorResponse`
- [ ] T023-P-02 Implement `pub fn replan(req: ReplanRequest) -> Result<ReplanResponse, ReplanError>`

## Phase 3 — main.rs

- [ ] T023-M-01 Hand-rolled arg parser (no clap)
- [ ] T023-M-02 `replan` subcommand: --request, --topology/--intent/--old-plan/--failed-nodes, stdin fallback
- [ ] T023-M-03 Map errors → exit codes per spec §3.3
- [ ] T023-M-04 `--version` and `--help` subcommands

## Phase 4 — tests

- [ ] T023-T-01 `replan_with_empty_blacklist_returns_old_plan`
- [ ] T023-T-02 `replan_after_node_pruning_produces_new_route`
- [ ] T023-T-03 `replan_with_no_survivors_returns_no_replacement`
- [ ] T023-T-04 `replan_with_empty_intent_returns_empty_intent_error`
- [ ] T023-T-05 `replan_request_json_round_trip`
- [ ] T023-T-06 `cli_end_to_end_via_stdin` (spawns binary)
- [ ] T023-T-07 `cli_handles_missing_request_file`
- [ ] T023-T-08 `cli_replan_with_failed_nodes_flag`
- [ ] T023-T-09 `cli_version_flag`
- [ ] T023-T-10 `cli_help_flag`

## Phase 5 — verify

- [ ] T023-V-01 `cargo build --release -p fabric-graph-cli`
- [ ] T023-V-02 `cargo test -p fabric-graph-cli` (all 10 tests pass)
- [ ] T023-V-03 `cargo test --workspace` (regression check, 162+ → 172+ expected)
- [ ] T023-V-04 `check_manifest.py` green
- [ ] T023-V-05 `check_json_schemas.py` / `check_openapi.py` / `check_links.py` green
- [ ] T023-V-06 End-to-end smoke: pipe JSON in, parse JSON out

## Phase 6 — commit + docs

- [ ] T023-C-01 Regenerate MANIFEST.sha256 (4 new entries)
- [ ] T023-C-02 Update specs/INDEX.md (+1 row)
- [ ] T023-C-03 Append ROADMAP.md R2 wedge #1 marker
- [ ] T023-C-04 Commit (1 feat + 1 docs)
- [ ] T023-C-05 Append WORKLOG entry
- [ ] T023-C-06 Append meta/PHENOTYPE_ARCHITECTURE.md addendum

## Cross-deps

- **Blocked by**: spec 020 (lease integration contract), ADR-0030 (failover model)
- **Blocks**: R2 wedge #2 (cmd/checker -checker-replan Go wiring)
- **Unlocks**: PF-WP-040 wire transport (R2 later)
