# Plan 023 — fabric-graph-cli replan

## Phase 0 (read authoritative source first, per ADR-0028)

Before opening the crate, these reads are mandatory:

1. `crates/fabric-graph/src/failover.rs` — `replan()` signature, `FailoverOutcome`, `FailoverError`
2. `crates/fabric-graph/src/model.rs` — `Topology`, `Intent`, `RoutePlan`, `NodeId` Serialize/Deserialize derives
3. `crates/fabric-graph/src/lib.rs` — re-exports used by external crates
4. `crates/fabric-graph/src/compile.rs` — what `compile()` returns (needed for error mapping)
5. `Cargo.toml` (workspace) — available deps, resolver = "2"
6. `crates/fabric-capability/src/locality.rs` — `LocalityTier` Serialize/Deserialize (to ensure request JSON round-trips)
7. `cmd/checker/main.go` — Go subprocess convention (stdout reserved for result, stderr for logs)

Phase 0 reads above complete in this turn (before any code in this crate).

## Phase 1 — scaffold

1. `mkdir crates/fabric-graph-cli/{src,tests}`
2. Author `Cargo.toml` — `fabric-graph = { workspace = true }`, `serde`, `serde_json`, `anyhow`, `thiserror`
3. Add `"crates/fabric-graph-cli"` to workspace `members` in root `Cargo.toml`
4. `pub mod request;` and `pub mod protocol;` in `src/lib.rs` (so integration tests can import)

## Phase 2 — write protocol.rs

Define:

```rust
#[derive(Serialize, Deserialize)]
pub struct ReplanRequest {
    pub topology: Topology,
    pub intent: Intent,
    pub old_plan: RoutePlan,
    pub failed_nodes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "PascalCase")]
pub enum ReplanResponse {
    Replaced { plan: RoutePlan },
    NoReplacement,
}

#[derive(Serialize, Deserialize)]
pub struct ReplanErrorResponse {
    pub error: String,
    pub message: String,
}
```

`pub fn replan(req: ReplanRequest) -> Result<ReplanResponse, ReplanError>` —
thin wrapper that parses `failed_nodes: Vec<String>` → `Vec<NodeId>`,
calls `failover::replan()`, maps to response.

## Phase 3 — write main.rs

Hand-rolled arg parser (no clap):

```rust
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match parse_subcommand(&args[1..]) {
        Subcommand::Replan { request } => run_replan(request),
        Subcommand::Version => { println!("fabric-graph-cli {}", env!("CARGO_PKG_VERSION")); ExitCode::SUCCESS }
        Subcommand::Help => { print_help(); ExitCode::SUCCESS }
        Subcommand::Flags(_) => { eprintln!("missing subcommand"); print_help(); ExitCode::from(2) }
    }
}
```

`run_replan`:
- If `--request <file>` provided, read the file.
- Else if `--topology`/`--intent`/`--old-plan`/`--failed-nodes` all provided, assemble.
- Else, read stdin.
- Call `protocol::replan(req)`.
- Serialize `ReplanResponse` or `ReplanErrorResponse` to stdout.
- Map errors to exit codes per spec §3.3.

## Phase 4 — tests

`src/protocol.rs` `#[cfg(test)] mod tests`:
1. `replan_with_empty_blacklist_returns_old_plan` — pass empty failed_nodes, expect Replaced with old plan
2. `replan_after_node_pruning_produces_new_route` — prune a, replan with failed=[a], expect Replaced using only b
3. `replan_with_no_survivors_returns_no_replacement` — empty topology, expect NoReplacement
4. `replan_with_empty_intent_returns_empty_intent_error` — empty intent name, expect Err(EmptyIntent)
5. `replan_request_json_round_trip` — serialize then deserialize a ReplanRequest, assert equal

`tests/replan_e2e.rs`:
6. `cli_end_to_end_via_stdin` — `Command::new(env!("CARGO_BIN_EXE_fabric-graph-cli")).arg("replan").write_stdin(json).assert().success()` — parse stdout JSON, assert Replaced
7. `cli_handles_missing_request_file` — `--request /nonexistent.json` → exit 2
8. `cli_replan_with_failed_nodes_flag` — uses `--topology --intent --old-plan --failed-nodes a,b` → exit 0
9. `cli_version_flag` — `--version` → exit 0, stdout contains version
10. `cli_help_flag` — `--help` → exit 0, stdout contains "Usage:"

## Phase 5 — verify

1. `cargo build --release -p fabric-graph-cli` — must succeed
2. `cargo test -p fabric-graph-cli` — all tests pass
3. `cargo test --workspace` — must remain green (162 → 167+ expected)
4. `python3 program/scripts/check_manifest.py` — must include new files
5. `python3 program/scripts/{check_json_schemas,check_openapi,check_links}.py` — must remain green
6. End-to-end operator smoke:
   ```bash
   cat > /tmp/req.json <<EOF
   {"topology":{...},"intent":{...},"old_plan":{...},"failed_nodes":["a"]}
   EOF
   ./target/release/fabric-graph-cli replan --request /tmp/req.json
   ```

## Phase 6 — commit + docs

1. Regenerate `MANIFEST.sha256` (4 new entries + 2 changed)
2. Update `specs/INDEX.md` (+1 row for spec 023)
3. Update `ROADMAP.md` R2 wedge #1 marked shipped
4. Commit (1 feat + 1 docs)
5. Append WORKLOG entry
6. Append `meta/PHENOTYPE_ARCHITECTURE.md` addendum

## Risk register

- **R1**: `cargo check -p fabric-graph-cli` hits a cascading-type error
  pattern (same as fabric-cli Tier 3). **Mitigation**: Phase 0 reads are
  exhaustive (5 files), and `Topology`/`Intent`/`RoutePlan` already derive
  Serialize/Deserialize. Unlikely. If it does fail, defer to a fresh
  context per ADR-0028 — do NOT grind.

- **R2**: The Go subprocess call adds latency (5-20ms typical).
  **Mitigation**: documented in spec §6; R3 candidate if measured.

- **R3**: Operator accidentally invokes the binary in a way that
  exposes the wrong failure mode. **Mitigation**: exit codes per spec §3.3
  are tight; integration tests cover each.

## Definition of done

- All 5 phases above complete.
- HEAD on phenotype-fabric is one commit ahead of `724e04b` (the R1 closeout).
- Both repos clean.
- All 4 spec checks green.
- 10+ tests added (5 unit + 5 integration per the plan).
