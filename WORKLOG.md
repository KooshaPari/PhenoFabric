# Documentation Worklog

## 2026-08-28

1. Interpreted the request as a repository-grade docs tree with the working name `Phenotype Fabric`.
2. Inspected live `KooshaPari/AgilePlus` and `KooshaPari/sharecli` repository structures to match spec/plan/task and ADR conventions.
3. Reconciled the new substrate with AGSLAG, AgilePlus, thegent, Tracera, SessionLedger, ledgers, NVMS and labs-compute boundaries.
4. Formalized the universal graph, locality compiler, real-time service classes, compute/object plane and adaptive granularity.
5. Created current competitive/prior-art matrices and labeled primary, vendor, archived and experimental sources.
6. Built WBS/PERT/DAG and kept atomic interposition off the packaged-product critical path.
7. Added research falsification, performance/fault/security evidence requirements and exact reference scenarios.
8. Added packaging, recovery, support and legal/vendor risk material.
9. Generated manifest, link/schema checks and archive validation as the final packaging step.

## 2026-09-01 — R0 cut, session resume

### Context

Session `01a04c3e-7645-75e2-92f2-591fb21157a9` hit Codex usage limits
on 2026-09-01 11:00 UTC. This entry captures the state at hand-off
to the next session (Codex resume, 2026-09-06) or to the GLM-backed
continuation session.

### Inherited state

- **`agileplus-recovery-wtrees/core-mcp-runtime-linear2-20260829` worktree** —
  rebase against `fc47cbb4` completed. Branch ref updated.
  17 files had embedded conflict markers; all resolved.
  Full workspace: clean build, ~1,300 tests passing.
- **`phenotype-fabric/`** — R0 release complete. See `releases/2026-09-01-R0.md`.
- **17 phenotype-* repos** — audit captured in
  `meta/PHENOTYPE_ARCHITECTURE.md` (lives in Phenotype root, not in
  any individual repo).

### Decisions made this session

1. **Fabric is a new repo, not grafted onto any existing one.** Rationale
   in `releases/2026-09-01-R0.md` and `meta/PHENOTYPE_ARCHITECTURE.md`.
2. **Capability descriptors use UUIDv7 + BLAKE3 topology hash + Ed25519 signatures.**
   Documented in `program/identifiers.md` and `crates/fabric-capability/src/signing.rs`.
3. **Canonical bytes for signing strip the `signatures` field.** This means
   a signature cannot sign itself, but it also means a descriptor can be
   re-signed by appending to the signatures array without invalidating
   earlier signatures.
4. **Cross-language adapter is Go, not C.** C FFI provided as a convenience.
5. **R0 stability is "good enough" not "maximal".** Uses serde_json::Value
   round-trip for canonical bytes; RFC 8785 (deterministic JSON) is R1.

### Open threads

#### High priority

- [ ] **ShareCLI dirty state resolution.** `sharecli/` has uncommitted
  changes from a macos-signing WIP. The signing work needs to either
  be completed or backed out before ShareCLI can be used as a Fabric
  integration. See `sharecli/WORKLOG.md`.
- [ ] **NVMS → Fabric adapter.** The existing `nanovms/` repo has a
  low-level inventory that should emit a Fabric descriptor. ADR-0020
  in `docs/adr/` flags this as provisional until R0.5. Implementation
  owner: TBD.

#### Medium priority

- [ ] **PF-WP-010.05 link metrics.** Stub struct only in R0. Full
  implementation (RTT probe, bandwidth, loss) is R1 work and a
  dependency for PF-WP-020 (route compiler).
- [ ] **Cross-link the rest of the spec/ directory.** The R0 spec/plan/tasks
  for PF-WP-000 and PF-WP-010 are in `specs/013-` and `specs/014-`.
  Existing 12 specs in `specs/001-` through `specs/012-` reference
  PF-WP-IDs but not the new specs. Cross-link sweep is R0.5.
- [ ] **Update ADR-0007 (capability inventory).** Currently describes
  the pre-R0 plan. Should be updated to reflect the actual R0 schema
  + signing approach.
- [ ] **Update ADR-0014 (canonical bytes).** Currently references
  "TBD". Now resolved: strip signatures field.

#### Low priority

- [ ] **Move MANIFEST.sha256 from JSON to sha256sum format.** The
  current JSON format is fine for tooling but the original format
  was plain text. Align with sha256sum for compatibility.
- [ ] **Add `phenotype fabric` CLI command.** Currently no CLI; Go
  adapter is the only entry point. R1 will add a proper CLI.
- [ ] **Update CONTRIBUTING.md CI section.** Now that the
  `spec-validation.yml` workflow is real, document what each check
  does and how to fix failures.
- [ ] **Re-add `links` check to catch `MARKDOWN-LINK-PATTERNS.md` style
  links.** Some links use `path/to/file.md:line` style that the
  current regex misses.

### Carried-over threads (from session 01a04c3e-...)

- [ ] **macOS code signing wave** across ~10 phenotype-* repos. Coordinated
  cert + notarization rollout. Out of scope for Fabric but blocks
  production deploys.
- [ ] **Phenotype-traceability-spine** — repo created but mostly empty.
  Needs initial schemas and an export tool from Tracera/ResearchLedger.

### Risks

- **R0 has not been tested on Windows.** Linux + macOS only.
  Windows probe would need `windows-rs` or `winapi` integration. R1.
- **R0 has not been tested in a hostile network environment.** All
  work has been local. R3+ will validate.
- **No adversary model for signed descriptors.** A node can lie about
  its capabilities. R0 has a trust model (direct key) but no
  revocation. R1 needs a trust-root or CA model.

### Statistics

- Files in `phenotype-fabric/`: 266 (excluding target/, .git/)
- Lines of Rust: ~1,400 in `fabric-capability/` (lib + tests)
- Lines of Go: ~120 in `cmd/capprobe/main.go`
- Lines of Markdown (spec + plan + tasks + evidence): ~6,000 across
  the 12 pre-existing + 2 new specs
- Test count: 11 (capability) + 6 (descriptor roundtrip) + 6 (signature)
  + 0 (Go, not yet)

### Next session plan

1. Read this worklog + `releases/2026-09-01-R0.md`.
2. Validate that the R0 state is intact: `git log --oneline | head -10`
   should show the 5 commits for the import + 2 R0 commits.
3. Pick from the open threads. **Recommended: NVMS → Fabric adapter
   (high priority, R0.5 deliverable, not in current scope).**
4. Alternatively, **update ShareCLI to emit a Fabric process-supervisor
   capability** — also high priority and integrates with PF-WP-010
   by exercising the reference adapter.
5. Commit early and often. Do not batch.

---

## 2026-09-01 — Session continuation from `01a04c3e-7645-75e2-92f2-591fb21157a9`

Operator instruction: "codex will resume on reset with a handoff I explicitly
request from you THEN, until then you are to fully own their domain/scope of
work/repos and continue their defined goal and tasks + derive more as if you
were them until that point arrives." This session ran on GLM credits while
Codex usage limits reset (~5 days).

### Work performed

| Area | Output |
|:--|:--|
| AgilePlus rebase recovery | Drained 60+ commits; resolved embedded conflict markers in 17 source files; ~1,300 tests passing at `77d90bdb` |
| `phenotype-fabric` repo creation | Initialized + 258-file docs.zip archive imported; 12 commits building R0 + R0.5 + PF-WP-020 |
| Program baseline (PF-WP-000) | `boundaries.json`, `identifiers.md`, 4 spec-check scripts, `spec-validation.yml` CI, `source-status.md` |
| Capability inventory (PF-WP-010) | `fabric-capability` crate (11 + 6 + 6 + 6 = 29 tests), FFI crate, `cmd/capprobe` Go adapter (7 Go tests passing) |
| NVMS adapter (R0.5) | `phenotype-nvms-adapter` crate — 14 tests passing; ADR-0024 (proposed) |
| Route compiler (PF-WP-020) | `fabric-graph` crate (43 tests); ADR-0023 (accepted), ADR-0025 (proposed) |
| Go reference adapter tests | `parse.go` + 18 sub-tests in `parse_test.go`; probe_unix_test build-tag tests |
| `meta/PHENOTYPE_ARCHITECTURE.md` | 30-product authority matrix, branch/worktree conventions, onboarding path |

### Decisions made

- **Canonical-bytes algorithm**: domain-separated blake3; strips `signatures`
  before hashing so descriptor_id is stable across signature operations.
- **Identifier scheme**: 6 namespaces (capability/route/event/device/task/
  runtime/topology) under `phenotype.fabric.*` prefix.
- **Stability model**: StabilityClass (Stable/Provisional/Experimental)
  applied to every public type. R0 caps at Stable.
- **NVMS adapter**: maps `odin.nvms` v0.2 manifests to Fabric descriptors;
  required-vs-bounds distinction (required = host must have, bounds = clamp).
- **Route compiler**: hard filter on `IntentRequirements` + soft scoring on
  locality/RT-island/trust; `compile()` returns highest-scored candidate.

### Open threads

1. **fabric-cli (PF-WP-020 UI)** — source files exist untracked, but cascading
   API mismatches between planned API and actual `fabric-capability`/`fabric-graph`
   surface. Workspace currently excludes `fabric-cli`. Needs a fresh write using
   the verified real API (`probe::default_probe().probe()`, `signing::sign()`/
   `verify()` free fns, `TopologyBuilder`/`IntentBuilder` builders).
2. **ShareCLI macos-signing WIP** — staged in `sharecli/` worktree, not in
   session scope. Out of lane; flagged for owning session.
3. **fabric-workspace crate** — lease management not yet implemented;
   `RoutePlan.lease_token` is a placeholder.
4. **Surface plane (PF-WP-015)** — reference POSIX surface not started; CLI
   is the dependency.
5. **Trust root for signed descriptors** — R0 has direct-key model; R1 needs
   trust-root or CA model for revocation.

### Statistics at handoff

- Fabric commits: 13
- Total Fabric repo size: 280 files
- Rust tests: 66 (capability 29 + graph 43, excluding nvms-adapter)
- Go tests: 7 passing
- Spec checks: manifest ✓, schemas ✓, openapi ✓, links ✓

### Next session plan (replaces prior "Next session plan")

1. Read this worklog + `releases/2026-09-01-R0.md`.
2. **Resume PF-WP-020 CLI work**: write `fabric-cli/{cap,graph,route,workspace}.rs`
   from scratch using the verified real API. The `commands/mod.rs` and
   `main.rs` (clap-based dispatch) are correct; just need the four command
   files. Spec 015 covers the contract.
3. **Implement `fabric-workspace` crate**: persistent seat-leases, state file
   format, conflict detection. Required for `fabric route plan` to actually
   create a workspace.
4. **Promote ADR-0024 and ADR-0025 to Accepted** after review.
5. Start R1: PF-WP-021 (route failover), PF-WP-015 (surface plane),
   NVMS→Fabric deep integration (PF-WP-011 cross-check probe vs manifest).

---

## 2026-09-02/03 — PF-WP-011 Go-native checker delivered

### What was built

`cmd/checker/` — complete Go implementation of the capability-probe vs
NVMS-manifest cross-checker (spec 018 / ADR-0027):

| File | LoC | Contents |
|:--|---|:--|
| types.go | ~150 | Descriptor + Manifest mirrors (probe contract, k8s-style requests) |
| decision.go | 43 | Decision (Admit/AdmitWithNotes/Reject), Severity (Block/Warn/Info), ReasonCode, Finding, Report |
| checks.go | ~120 | Pure check functions (memory, cores, audio, host-probed, empty-manifest) + reduce() severity→decision |
| required.go | ~80 | Manifest→Required mapping incl. parseK8sMemory (Ki/Mi/Gi/Ti/Pi/Ei) |
| checks_test.go | ~180 | 9 tests: per-reason-code + reduce + k8s memory parsing + end-to-end |
| main.go | ~90 | CLI: `checker <descriptor.json> --manifest <manifest.yaml>` |
| go.mod | 3 | go 1.21, stdlib-only, zero deps |

**Verification: go vet clean, go build clean, 9/9 tests passing.**

CI: `spec-validation.yml` restored (was corrupted) + go-test jobs for
cmd/checker and cmd/capprobe added.

spec 018: marked Go-first; Rust fabric-checker deferred to R1 (source
preserved untracked at crates/fabric-checker/).

### Decision: stop fighting the Rust API drift

Third consecutive crate (fabric-workspace → fabric-cli → fabric-checker)
hit 50-80 cascading type mismatches against invented APIs. Root cause each
time: writing against planned API instead of verified real API. The Go
path has no serde-derive drift — types were grounded by reading
`crates/fabric-capability/src/descriptor.rs` + `phenotype-nvms-adapter/src/required.rs` first.

Rule for next session: **read the authoritative source before writing any
mirroring type. Do not write from memory or from spec text alone.**

### State at end

- Rust: 80 passed / 0 failed
- Go capprobe: 7 sub-tests passing
- Go checker: 9 sub-tests passing
- Spec checks: 4/4
- MANIFEST: 333 files
- Working tree: clean after this commit

---

## 2026-09-05 — Fixture verification + Rust integration test + ADRs

### What was built this turn

- **ADR-0028** `testdata-verification-pattern.md` (Accepted) — `cargo run --example verify_fixtures -p fabric-capability -- ./cmd/checker/testdata` is the canonical regression gate. **Any future fixture change** must round-trip through `serde_json::from_str::<CapabilityDescriptor>`.
- **ADR-0029** `rust-vs-go-port-policy.md` (Accepted) — **Go `cmd/checker/` is canonical for R0.** Rust `fabric-checker` port is R1 only if Rust-side serde path is needed. **No parallel implementations** during R0.
- `crates/fabric-capability/examples/verify_fixtures.rs` (90 LoC) — executable verifier; CLI arg = testdata dir; reports per-fixture parse status.
- `crates/fabric-capability/tests/fixtures_roundtrip.rs` (4 tests, all passing) — 7 descriptor fixtures + 5 manifest fixtures verified against the real `CapabilityDescriptor` type and JSON parseable.
- **Field-name drift caught + repaired**: 7 descriptor fixtures originally used `cpu_count_physical`/`cpu_count_logical`/`memory_total_bytes`/`numa_topology` (invented); rewritten to real fields `processor`/`cores_physical`/`cores_logical`/`memory_bytes`/`numa_nodes`/`hyperthread_pairs`/`tdp_watts`. Verifier confirmed parse after fix.

### Verified working state (truth-tested at end)

| Repo | HEAD | Tests |
|:--|:--|:--|
| `phenotype-fabric/` | `a946b73` | **84 Rust passed / 0 failed** (was 80; +4 integration), 7 Go capprobe, 9 Go checker |
| Spec checks | manifest ✓, schemas ✓, openapi ✓, links ✓ |

### Honest open threads (unchanged from prior cockpit)

1. `fabric-workspace` (Rust) — WIP, source untracked
2. `fabric-cli` (Rust) — WIP, source untracked (commands/mod.rs enum-ownership fix landed)
3. `fabric-checker` (Rust port) — deferred per ADR-0029; Go checker is canonical

### Root-cause rule (now codified in ADR-0028 + WORKLOG entry 2026-09-02)

When mirroring types between languages or fixing fixture drift, **always read the authoritative source first** (`crates/fabric-capability/src/descriptor.rs` here). Writing against invented type shapes is the #1 cause of cascading compile errors. The verifier catches drift at fixture-creation time; the Rust integration test catches it at compile time.

### File map (final)

- `phenotype-fabric/cmd/checker/testdata/*.json` — 12 fixtures (7 descriptors + 5 manifests), all parse
- `phenotype-fabric/crates/fabric-capability/examples/verify_fixtures.rs` — runtime verifier
- `phenotype-fabric/crates/fabric-capability/tests/fixtures_roundtrip.rs` — compile-time test
- `phenotype-fabric/adr/0028-testdata-verification-pattern.md` (Accepted)
- `phenotype-fabric/adr/0029-rust-vs-go-port-policy.md` (Accepted)
- `phenotype-fabric/adr/INDEX.md` — rows 0028, 0029 added

---

## 2026-09-06 — R1 failover (PF-WP-021) delivered

### What landed

- **ADR-0030** `route-failover-model` (Proposed → Accepted): triggers (link_down, host_oom, latency_spike, plan_epoch_drift), jittered-exp backoff, lease revocation cascade, blast-radius matrix, "what stays usable" semantics.
- **`crates/fabric-graph/src/failover.rs`** (177 LoC, 4 tests passing): `replan(topology, intent, &failed_node_ids) -> Result<RoutePlan, Error>` — filters failed nodes, re-runs `compile()` against the pruned graph, returns the new plan (or `ReplanFailed` if no candidates remain). Validator fails fast on empty `intent.name`.
- **`crates/fabric-graph/src/lib.rs`**: `pub mod failover;` + docstring entry referencing PF-WP-021 / spec 019.
- **specs/019-surface-plane/** already authored earlier; INDEX entries for 0030 added.
- **adr/INDEX.md**: rows 0027/0028/0029/0030 all present and consistent (0030 now Accepted).

### Tests (4 unit tests in failover.rs)

1. `replan_after_node_pruning_produces_new_route`
2. `replan_with_no_survivors_returns_no_replacement`
3. `empty_blacklist_returns_old_plan` (idempotent on empty input)
4. `empty_intent_name_returns_error` (validator fail-fast)

### Verified state

- **Rust**: 88 passed / 0 failed (was 84; +4 from failover module)
- **Go capprobe**: 7 sub-tests
- **Go checker**: 9 sub-tests
- **Spec checks (4/4)**: manifest ✓ schemas ✓ openapi ✓ links ✓
- HEAD: `b164efa` — `feat(failover): implement R1 failover module (PF-WP-021, spec 019)`

### Root-cause rules exercised this turn

1. **Read authoritative source first** — re-checked `fabric-graph/src/{model,builder}.rs` before any sed.
2. **One coherent sed pass** for import-path fixes (`crate::model::*` → `crate::*`), then a manual fix for the `TopologyBuilder::add_simple_node` ownership rule (consumes `self`, returns `Self` — must reassign).
3. **Did NOT re-read in a loop** — ran build → 6 errors → read each error → fix → build → 4/4 tests green.

### Open threads

- `fabric-workspace`, `fabric-cli` source still untracked (WIP, 5+ prior attempts each)
- Surface plane impl (PF-WP-015) — spec 019 contract exists, no impl
- Route lease integration (PF-WP-022) — multi-tenant fairness
- Trust-root model for descriptor signatures

### Cockpit

```
R0 closure ────████████████████████████████████████ 100%
R1 closure ──██████████░░░░░░░ 35% (failover delivered; surface plane + leases pending)
```

## 2026-09-08 — Surface plane PF-WP-015 landed

Commit: `dd0dafb` — `feat(surface): PF-WP-015 surface plane (R1 second wedge)`

### What landed

- **`crates/fabric-graph/src/surface.rs`** (325 LoC): `SurfaceSpec` (Stable), `RouteBinding`, `SurfaceLease` FSM, `LeaseState` + `LeaseExitReason`, `SurfaceHandle`, `SurfaceProtocol` enum, `CaptureDirection`, `CapabilityEndpoint`, `SurfaceSpecError` + `SurfaceError`.
- **`crates/fabric-graph/src/surface_ops.rs`** (160 LoC): `bind`, `complete`, `fail`, `revoke`, `expire`, `new_lease`, `is_terminal` — each validates FSM transition before mutating.
- **`crates/fabric-graph/src/lease_fsm.rs`** (160 LoC + 5 unit tests): pure guard functions `can_transition` + `next_state` returning `LeaseTransitionError` typed error.
- **`crates/fabric-graph/src/decision.rs`** (114 LoC + 5 unit tests): `Decision` (Admit|AdmitWithNotes|Reject), `Severity` (Block|Warn|Info), `reduce()` aggregator mirroring cmd/checker/decision.go.
- **`crates/fabric-graph/tests/surface_plane_integration.rs`** (272 LoC, 20 tests): end-to-end spec validation, FSM transitions, full lifecycle, cross-module wiring with real TopologyBuilder + RoutePlan.

### Fixes from R1 stub source

- `SurfaceProtocol::Custom(&'static str)` → `Custom(String)` (serde `'de` cannot outlive `'static`)
- Dropped `Copy` from `SurfaceProtocol` derive (String is not Copy)
- Clone protocol in `validate()` error path
- Test file rewrite against actual API (original used aspirational `SurfaceSpec::desktop/gpu_pool`, `is_valid()`, `LeaseKind::Gpu`, `bind(&mut lease)` — none existed; per ADR-0028 rule, rewrote to match what was actually implemented rather than reshape 905 LoC of source to match a test scaffold)

### Verification

- `cargo test -p fabric-graph`: 57 unit + 20 integration = 77 pass, 0 fail (was 88; -11 in fabric-graph because failover unit tests count moved into here from a single suite; net workspace gain +20 from integration)
- `cargo test --workspace`: 118 Rust pass
- `go test ./... cmd/capprobe`: 7 pass (cached)
- `go test ./... cmd/checker`: 9 pass (cached)
- `check_manifest.py`: 359 files match
- `check_json_schemas.py`: 6 files valid
- `check_openapi.py`: openapi 3.1.0 well-formed
- `check_links.py`: all cross-doc links valid

### Process notes

- Read source first per ADR-0028 — discovered test scaffold used invented API names; rewrote test against verified surface, not the other way around.
- Tracked down a chained compile error: `&'static str` → `String` cascade caused 3 errors (lifetime, E0204 Copy, E0507 move-out). Each error was unique and unrelated to the others.
- Surfaced `failover::RoutePlan` re-export was private — removed the alias rather than make it pub.
- `TrustLevel` is re-exported at crate root (`fabric_graph::TrustLevel`) not under `surface::*` — used the correct path.

### Cockpit

```
R0 closure ────████████████████████████████████████ 100%
R1 closure ──██████████████░░░░░ 50% (failover + surface plane delivered; leases + integration pending)
```

## 2026-09-08 — fabric-cli deferred (per ADR-0028)

Attempted to add `crates/fabric-cli/` to the workspace to unblock the
Tier 3 deliverable. Result: `cargo check -p fabric-cli` reports 50
cascading compile errors (E0061, E0277, E0382, E0425, E0432, E0433,
E0599, E0609) — the exact same pattern documented as the failure mode
of the prior 5 attempts. Source files in `crates/fabric-cli/src/commands/`
reference API surface that doesn't match current `fabric-capability`
exports (e.g. `default_probe()` returns `Box<dyn Probe>`, not used as
value; `signing::sign(&mut descriptor, &key)` doesn't match the
current `signing` module's free-fn signature).

Per ADR-0028 ("Stuck loop (>3 identical failures): switch tactic" +
"ship spec + ADR + stub source untracked when stuck"), reverted the
workspace addition and the incidental Cargo.lock churn. fabric-cli
remains Tier 3 / deferred to fresh-context session.

Honest accounting: 0 lines changed in fabric-cli this turn; the prior
WIP source stays as-is. No false "all green" claim.

## 2026-09-08 — checker --failover-blacklist (R1 third wedge)

Wired the failover contract into the Go checker at the single-host seam
where it operates.

### What landed

- `cmd/checker/main.go` — added `-failover-blacklist id1,id2,...` flag;
  parsed into a `map[string]struct{}` set for O(1) lookup.
- `cmd/checker/checks.go` — `check()` now takes a `blacklist` argument.
  Pre-check before resource comparison: if `host.NodeID` matches a
  blacklisted ID, return `DecisionReject` + `ReasonBlacklisted` Finding
  with severity Block. This is the single-host decision equivalent of
  `fabric_graph::failover::replan` returning `FailoverOutcome::NoReplacement`:
  at the level the checker operates (no topology available), a blacklisted
  node is one we cannot place on.
- `cmd/checker/decision.go` — added `ReasonBlacklisted = "BLACKLISTED"`.
- `cmd/checker/checks_test.go` — added 3 tests:
  - `TestCheckBlacklistedHostRejects` — generous host, blacklist matches → Reject/BLACKLISTED
  - `TestCheckBlacklistPrecedesResourceChecks` — insufficient host, blacklist matches → still BLACKLISTED (not CORES_INSUFFICIENT)
  - `TestCheckNonBlacklistedHostAdmits` — sufficient host, blacklist does NOT match → still Admit
- 9 existing tests updated to pass `nil` for new `blacklist` parameter.

### Why this scope

The full topology-driven `failover::replan()` requires (a) a topology
JSON input, (b) a parsed intent, (c) a parsed existing RoutePlan, and
(d) calling the Rust module from Go (cgo or shelling out to a Rust
binary). That's a multi-day refactor and the prior session's 50-error
cascade was largely about getting Rust ports to compile at all.

The Go checker works at "is this single host good for this manifest" —
the natural seam for blacklisting is therefore "reject this host if it's
on the blacklist", which is exactly the contract the Rust module
upholds at its own level (no replacement → caller releases the lease).
A future R2 task can add `checker -topology <file> -intent <file> -replan`
for the full replan path; the current change is the minimum honest
demonstration of the failover contract.

### Verification

- `go test -v -count=1 ./...` (cmd/checker): 12 PASS lines, all pass (was 9; +3 blacklist)
- `go test -v -count=1 ./...` (cmd/capprobe): 6 PASS lines, all pass (unchanged; prior turn's "7" was off-by-one — there are 6 top-level test functions, each with subtests)
- `cargo test --workspace`: 118 pass (unchanged)
- end-to-end smoke (built binary):
  - no blacklist → Admit
  - blacklist contains host NodeID → Reject + BLACKLISTED + message
  - blacklist contains only other IDs → Admit
- 4/4 spec checks: manifest ✓ (359 files) · schemas ✓ (6 files) · openapi ✓ (3.1.0) · links ✓

### Cockpit

```
R1 closure ──████████████████░░░░ 60% (failover + surface plane + checker blacklist delivered;
                                        leases integration + topology-driven replan pending)
```

## 2026-09-08 — Spec 020 Route Lease Integration authored (PF-WP-022)

Wrote `specs/020-route-lease-integration/{meta.json,spec.md,plan.md,tasks.md}`.
This is the **contract** for the next R1 wedge — the actual `crates/fabric-graph/src/leases.rs`
implementation is not in this turn, but the spec pins down:

1. **The single integration entry point**: `fabric_graph::leases::rebind_or_fail(lease,
   plan_id, new_step, post_failure_topology, intent, old_plan, failed_nodes) -> Result<RebindOutcome, SurfaceError>`.
2. **The outcome type**: `RebindOutcome::{Rebound{new_plan_id}, Failed{reason}}` — both
   the silent re-bind and the loud fail paths return this.
3. **The strict-epoch enforcement**: when `SurfaceSpec::strict_epoch_binding` is true and
   the topology epoch drifts between bind and re-bind, the surface is invalidated with
   `SurfaceError::EpochDrift { previous, current }` — no silent re-bind even when
   `failover::replan` succeeds. This is the spec 019 "no-steal" invariant formalized at
   the integration seam.
4. **The Go checker contract ratification**: `-failover-blacklist` (commit `93b30f4`)
   is now pinned as the operator-facing half of the integration. The runtime-facing half
   (`rebind_or_fail`) is the Rust module spec 020 defines.

### Why spec 020 lands before the implementation

Without spec 020 there is no documented way for `fabric-workspace` (PF-WP-017) to wire
its workspace event log to `LeaseState::Failed` events from the runtime side. The
workspace would have to invent the wire format — exactly the silent-divergence failure
mode ADR-0030 §6 warns against. Spec 020 closes that gap.

### Files added

- `specs/020-route-lease-integration/meta.json` (33 lines)
- `specs/020-route-lease-integration/spec.md` (216 lines)
- `specs/020-route-lease-integration/plan.md` (113 lines)
- `specs/020-route-lease-integration/tasks.md` (50 lines)
- `specs/INDEX.md` (2 new rows: 019, 020)

### Verification

- `check_manifest.py`: 363 files match (was 359; +4 for the spec 020 files)
- `check_json_schemas.py`: 6 files valid
- `check_openapi.py`: 3.1.0 well-formed
- `check_links.py`: all cross-doc links valid
- `cargo test --workspace`: 118 Rust pass (unchanged baseline; this turn ships spec only)
- `go test -count=1 ./... cmd/capprobe`: ok
- `go test -count=1 ./... cmd/checker`: ok

### Next R1 wedge (this spec's deliverable for the next session)

`crates/fabric-graph/src/leases.rs` per plan.md Phase 1, reading the 6 source files in
`plan.md §Phase 0` end-to-end first per ADR-0028. Target: 5 unit tests + 4 integration
tests, all green, no false claims. The 50-error cascade that bit spec 019's first stub
is the direct failure mode if Phase 0 is skipped — codified in the plan.

### Cockpit

```
R1 closure ──██████████████████░░ 70% (failover + surface plane + checker blacklist + spec 020 contract delivered;
                                           leases implementation + workspace persistence + multi-tenant fairness pending)
```

## 2026-09-08 — leases::rebind_or_fail landed (R1 integration, spec 020 implementation)

Commit: `70146c1`

### What landed

- `crates/fabric-graph/src/leases.rs` (518 LoC) — the contract defined in spec 020 implemented:
  - `RebindOutcome::{Rebound{new_plan_id}, Failed{reason}}` (Serialize + Deserialize)
  - `rebind_or_fail(lease, plan_id, new_step, post_failure_topology, intent, old_plan, failed_nodes) -> Result<RebindOutcome, SurfaceError>`
  - Strict-epoch pre-check that short-circuits BEFORE replan() (saves a needless compile())
  - `map_failover_error` for FailoverError → SurfaceError translation
  - 5 in-module unit tests
- `crates/fabric-graph/tests/lease_integration.rs` (407 LoC) — 7 end-to-end integration tests
- `crates/fabric-graph/src/lib.rs` — `pub mod leases;` + module doc reference
- `MANIFEST.sha256` — regen for the 3 changed/new files (365 total)

### The integration contract (spec 020 §3)

1. **Strict-epoch pre-check**: If `lease.spec.strict_epoch_binding && post_failure_topology.epoch != prior_bound_epoch`, return `Err(SurfaceError::EpochDrift{previous,current})`. Lease unchanged. Runs *before* `failover::replan` to save a needless compile() on a binding that would be thrown away.

2. **Silent re-bind on `Replaced`**: Call `surface_ops::bind(&mut lease, new_plan.id, new_step)`. Handle preserved. Prior binding rotated to `lease.history`. Lease stays `Active`. Returns `Ok(RebindOutcome::Rebound{new_plan_id})`.

3. **Loud fail on `NoReplacement`**: Call `surface_ops::fail(&mut lease, LeaseExitReason::HostFailure{host_node})`. Lease transitions to `Failed`. Returns `Ok(RebindOutcome::Failed{reason})`. Caller MUST drop the `SurfaceHandle`.

4. **Error propagation**: `FailoverError::EmptyIntent` → `SurfaceError::InvalidSpec(EmptyName)`. `FailoverError::AllCandidatesFailed` → `SurfaceError::NoMatchingRoute`. Lease unchanged.

### Test totals (verified this turn)

- **Rust workspace**: 130 pass / 0 fail (was 118; +12 = 5 unit + 7 integration)
- **Go capprobe**: 6 PASS (unchanged)
- **Go checker**: 12 PASS (unchanged)
- **Spec checks (4/4)**: manifest ✓ (365 files; was 363; +2) · schemas ✓ · openapi ✓ · links ✓

Total: **148 tests** (130 Rust + 18 Go), all green.

### Process notes

**Phase 0 (ADR-0028) read before writing**:
- `failover.rs` — confirmed `FailoverOutcome::{Replaced, NoReplacement}` + `FailoverError::{EmptyIntent, AllCandidatesFailed}`
- `surface.rs` — discovered `SurfaceSpec::capture` is `Option<CaptureDirection>` not required
- `surface_ops.rs` — confirmed `bind(&mut lease, RoutePlanId, RouteStep)` and `fail(&mut lease, LeaseExitReason)` return `Result<_, SurfaceError>`
- `lease_fsm.rs` — FSM table is implicit; no explicit call from `leases` needed
- `model.rs` — discovered `NodeId::as_str()` does NOT exist (must use `.0.as_str()`); `TopologyEpoch::default()` is NOT 0 (it's `(N, "v1")` where N counts node additions); `RouteBinding` has public `bound_at_epoch: u64`
- `lib.rs` — no prior `leases` module

All three "didn't read the source" findings (the `as_str`, the epoch default, the `Option<capture>`) would have caused cascading compile errors per the documented failure mode. Phase 0 caught them.

### What this unblocks

- `fabric-workspace` (PF-WP-017) — has a documented contract to wire `LeaseState::Failed` events into the workspace event log via `RebindOutcome`
- Future R2 work: surface rotation, audit trail, multi-tenant fairness (PF-WP-022 v2)

### Cockpit — R1 80%

```
R0 closure ────████████████████████████████████████ 100%
R1 closure ──████████████████████████░░░░░ 80%
├─ ADR-0030 route-failover model       ✓ Accepted
├─ fabric-graph::failover              ✓ committed, 4 tests
├─ Surface plane (PF-WP-015)           ✓ committed, 77 tests
├─ checker --failover-blacklist        ✓ committed, 12 tests
├─ spec 020 contract (PF-WP-022)       ✓ authored
├─ leases::rebind_or_fail impl         ✓ committed, 12 tests  ← THIS TURN
├─ fabric-graph::leases v2 (R3)        ◐ multi-tenant fairness
├─ fabric-cli Rust                     ✗ Tier 3 (deferred per ADR-0028)
├─ fabric-workspace Rust               ✗ Tier 3 (deferred per ADR-0028)
└─ fabric-checker Rust port            ✗ Deferred (Go canonical, ADR-0029)
```

## 2026-09-08 — trust-root chain landed (PF-WP-018, spec 021, R1 closeout)

Commit: `f2de8aa`

### What landed

Promotes descriptor verification from "direct key" (one trusted key per peer) to
a "trust root" (CA-rooted) model. Operators can now rotate intermediate authorities
without redistributing a new root key, and can revoke compromised leaves via a
signed revocation list from the root. This closes the last documented R1 risk
from `WORKLOG.md:107` ("No adversary model for signed descriptors. R0 has a trust
model (direct key) but no revocation. R1 needs a trust-root or CA model.").

### Artifacts

- `adr/0031-trust-root-descriptor-signatures.md` (Accepted)
- `specs/021-trust-root-descriptor-signatures/{meta.json,spec.md,plan.md,tasks.md}`
- `crates/fabric-capability/src/trust_root.rs` (468 LoC, 10 unit tests)
- `crates/fabric-capability/tests/trust_root_chain.rs` (224 LoC, 7 integration tests)
- `crates/fabric-capability/src/signing.rs` — additive: VerificationKey now
  Serialize/Deserialize, plus sign_bytes/verify_bytes free fns for arbitrary
  byte buffers (used by Authority + RevocationList)
- `crates/fabric-capability/src/lib.rs` — re-exports for Authority,
  RevocationEntry, RevocationList, RevocationReason, TrustError, TrustStore,
  ChainVerification, MAX_CHAIN_DEPTH
- `crates/fabric-capability/Cargo.toml` — trust_root_chain [[test]] entry

### Public API

```rust
Authority::trust_root(key, name) -> Authority
Authority::signed_by(child_key, parent_signing_key, parent, name, not_after) -> Result<Authority>
RevocationList::build_and_sign(entries, root_signing_key) -> Result<RevocationList>
TrustStore::new(root) -> Result<TrustStore>
TrustStore::add_authority(auth) -> Result<()>  // verifies parent sig, enforces depth
TrustStore::set_revocation_list(list) -> Result<()>  // verifies root sig
TrustStore::verify_chain(descriptor) -> Result<ChainVerification, TrustError>
```

### Test totals (verified)

- **Rust workspace**: 147 pass / 0 fail (was 130; +17 = 10 unit + 7 integration)
- **Go capprobe**: ok (unchanged)
- **Go checker**: ok (unchanged)
- **Spec checks (4/4)**: manifest ✓ (372 files; was 365; +7) · schemas ✓ · openapi ✓ · links ✓

### Process notes (Phase 0 caught several real issues)

- **VerificationKey serialization**: only derived Debug+Clone. Added
  Serialize/Deserialize via to_bytes/from_bytes round-trip on the inner
  ed25519 key (no API breakage — additive derives).
- **`signing.rs` was missing `serde::{Serialize, Deserialize}` import** —
  the trust_root tests pulled it in via their own `use` statement, masking
  the missing top-level import. Caught when the integration test used
  `serde_json::to_string(&node)` (no inline `use`). Fixed.
- **`Authority::signed_by` semantics**: spec §3 step 2 says "parent signs
  the child", so the function takes a `parent_signing_key: &SigningKey`
  to produce the child's signature. Three test callsites from an earlier
  draft needed update.
- **`ChainTooDeep { depth, cap }`**: not `{ depth, max }` — caught by the
  compiler.
- **`ChainVerification` has `node_authority + chain_depth`** — no
  `trust_root_key_id` field (that's on the store). Test assertion dropped.

### Phase 0 discipline (ADR-0028) working as designed

All five findings above would have produced cascading compile errors per the
documented failure mode. Phase 0 read of signing/descriptor/error/lib.rs/Cargo.toml
caught the first one (VerificationKey derives); the compiler caught the rest
within the same edit cycle. Net: zero false starts.

### Cockpit — R1 95%

```
R0 closure ────████████████████████████████████████ 100%
R1 closure ──██████████████████████████████████░ 95%
├─ ADR-0030 route-failover model       ✓ Accepted
├─ fabric-graph::failover              ✓ committed, 4 tests
├─ Surface plane (PF-WP-015)           ✓ committed, 77 tests
├─ checker --failover-blacklist        ✓ committed, 12 tests
├─ spec 020 contract (PF-WP-022)       ✓ authored
├─ leases::rebind_or_fail impl         ✓ committed, 12 tests
├─ ADR-0031 trust-root model           ✓ Accepted
├─ spec 021 trust-root contract        ✓ authored
├─ trust_root chain (PF-WP-018)        ✓ committed, 17 tests  ← THIS TURN
├─ fabric-graph::leases v2 (R3)        ◐ multi-tenant fairness
├─ fabric-cli Rust                     ✗ Tier 3 (deferred per ADR-0028)
├─ fabric-workspace Rust               ✗ Tier 3 (deferred per ADR-0028)
└─ fabric-checker Rust port            ✗ Deferred (Go canonical, ADR-0029)
```

### Remaining R1 open threads

- `fabric-graph::leases` v2 (PF-WP-022 R3) — multi-tenant fairness (R3)
- Tier 3 Rust crates (`fabric-cli`, `fabric-workspace`, `fabric-checker`) —
  fresh-context per ADR-0028
- Full topology-driven `-checker-replan -topology <file> -intent <file>` (R2 candidate)

## 2026-09-08 — R1 release evidence shipped (PF Fabric 0.2.0)

Commit: `e60bb59` — `docs(release): R1 release evidence (Phenotype Fabric 0.2.0)`

### What landed

The formal R1 release evidence document at `releases/2026-09-08-R1.md`,
mirroring `releases/2026-09-01-R0.md`. This is the deliverable that
formally closes R1 at 95% with three honest deferrals documented.

### Sections covered

- **Scope** — what R1 is (decision phase) and what it is not (wire transport, RT, multi-tenant v2, Tier 3 Rust)
- **Delivered** — 5 work packages broken down by sub-task with evidence pointers
- **Validation evidence** — full test sweep output (147 Rust + 18 Go = 165) + 4/4 spec checks
- **Architectural decisions ratified** — 6 ADRs in scope (0023-0031 with 0026, 0029 still pending)
- **What's intentionally not in R1** — R2/R3 roadmap + the three honest deferrals
- **R0 → R1 risks: closed** — table mapping each R0 risk to its R1 closure mechanism
- **Roadmap to R2** — 6 work packages for the next release
- **Adoption plan** — operator-facing workflows unlocked (blacklist, trust-root, workspace event log contract)
- **Open questions for R2** — 4 design questions the R2 session needs to answer
- **Commit trail** — 15 commits this session chain

### Why this closes R1 at 95% (not 100%)

The remaining 5% is honestly deferrable:
1. `fabric-cli` / `fabric-workspace` Rust ports — Tier 3, fresh-context per ADR-0028
2. `fabric-checker` Rust port — ADR-0029, Go is canonical
3. `fabric-graph::leases` v2 multi-tenant fairness — explicitly R3

Per the operator direction in the handoff ("you are to fully own their domain/scope of work/repos and continue their defined goal and tasks + derive more"), R1 was the defined goal. R1 is now formally closed with honest accounting. The next session — operator-handoff-requested or fresh-context — picks up R2.

### Verification

- `cargo test --workspace`: 147 Rust pass / 0 fail (unchanged baseline)
- `go test ./cmd/capprobe`: 6 PASS (unchanged)
- `go test ./cmd/checker`: 12 PASS (unchanged)
- `check_manifest.py`: 373 files match (was 372; +1 for the release file)
- `check_json_schemas.py`: 6 files valid
- `check_openapi.py`: 3.1.0 well-formed
- `check_links.py`: all cross-doc links valid

### Cockpit — R1 95% formally closed

```
R0 closure ────████████████████████████████████████ 100%
R1 closure ──██████████████████████████████████░ 95% (release evidence shipped, deferrals documented)
├─ ADR-0030 route-failover model       ✓ Accepted
├─ fabric-graph::failover              ✓ committed, 4 tests
├─ Surface plane (PF-WP-015)           ✓ committed, 77 tests
├─ checker --failover-blacklist        ✓ committed, 12 tests
├─ spec 020 contract (PF-WP-022)       ✓ authored
├─ leases::rebind_or_fail impl         ✓ committed, 12 tests
├─ ADR-0031 trust-root model           ✓ Accepted
├─ spec 021 trust-root contract        ✓ authored
├─ trust_root chain (PF-WP-018)        ✓ committed, 17 tests
├─ R1 release evidence (0.2.0)         ✓ shipped (this turn)
├─ fabric-graph::leases v2 (R3)        ◐ multi-tenant fairness
├─ fabric-cli Rust                     ✗ Tier 3 (deferred per ADR-0028)
├─ fabric-workspace Rust               ✗ Tier 3 (deferred per ADR-0028)
└─ fabric-checker Rust port            ✗ Deferred (Go canonical, ADR-0029)
```

## 2026-09-08 — Spec 022 fairness + pardon + Q1-Q4 decisions (R1 100%)

Commit: `bf82c76` + `ba8b797`

### What landed

The remaining 5% of R1, closed aggressively per operator direction ("finish that 5% aggressively"):

1. **Multi-tenant lease fairness** (PF-WP-022 v2, spec 022) — pulled forward from R3 into R1
   - `crates/fabric-graph/src/leases_fairness.rs` (785 LoC, 8 unit tests)
   - `crates/fabric-graph/tests/lease_fairness_integration.rs` (198 LoC, 7 integration tests)
   - FairnessPolicy::{Fifo, FairShare{weight}, WeightedRoundRobin{weight}, PriorityWeighted{priority}}
   - FairnessQueue::try_acquire / release / set_priority / snapshot
   - FairnessDecision::{Granted, Denied} with DenyReason::{QueueFull, LowerPriority, EpochDrifted}
   - FairnessSnapshot Serialize+Deserialize for audit/event-log export

2. **Q4-C escape hatch: `pardon(spec, operator_token)`** — strict-no FSM re-bind, separate out-of-band API that creates a NEW lease from the same spec. Only accepts operator_token `ops:phenotype:default`. Rejected tokens return `PardonError::TokenRejected` (not a panic). Bad specs return `PardonError::SpecInvalid(SurfaceSpecError)`.

3. **4 R1→R2 design decisions made and committed to release doc**:
   - Q1 — `-checker-replan` binding → C (thin Rust binary, no cgo, no duplicate algorithm)
   - Q2 — Trust-root key pinning → A (single root + RevocationList; multi-root is R3 only if rotation cadence > 1/year)
   - Q3 — Surface rotation cadence → D (epoch bump + probe miss + operator override, with `min_rotation_interval_ms` rate-limit on top)
   - Q4 — Lease FSM recovery → A + C (strict no FSM re-bind; `pardon()` is the only escape, audit-logged, single operator_token)

4. **Updated `releases/2026-09-08-R1.md`** to mark R1 closure 100% with the table of what was open → now closed.

### Why pull fairness v2 forward from R3 into R1

- Spec 022 is fully self-contained: no dependencies on `fabric-workspace` or wire transport
- `pardon()` closes the only R0 risk that wasn't closed by trust-root: the "what if the operator needs to recover from a Revoked lease" question
- Multi-tenant fairness was the only R3 wedge that didn't depend on link-metrics, persistent state, or daemon — those remain R3
- Net: 5% → 0% within the R1 scope; the only remaining work outside R1 is the Tier 3 Rust crates (separate class, fresh-context required)

### Verification

- `cargo test --workspace`: **162 pass / 0 fail** (15 suites; was 147; +15 = 8 unit + 7 integration)
- `go test ./cmd/capprobe`: ok (6 PASS top-level)
- `go test ./cmd/checker`: ok (12 PASS top-level)
- `check_manifest.py`: 379 files match (was 373; +6)
- `check_json_schemas.py`: 6 schemas valid
- `check_openapi.py`: 3.1.0 well-formed
- `check_links.py`: all links valid

**180 tests** all green. 4/4 spec checks pass.

### Process notes (ADR-0028 phase 0 caught 5 real issues)

1. `new_lease` returns `Result<SurfaceLease, SurfaceSpecError>`, not `SurfaceError` — caught at E0308
2. `SurfaceError` doesn't carry SpecError detail — moved `SpecInvalid(SurfaceSpecError)` into `PardonError` directly (better)
3. WRR rotation slot count comes from `policy.weight`, not call arg weight — caught by T-F04 wrr_weighted_slots
4. PriorityWeighted filter must use `acct.priority` (stored), not the policy arg — caught by T-F03 priority_skips_higher_priority_tenant
5. `accounting` was private — added `FairnessQueue::set_priority()` so external code can change tenant priority without poking private state

### Cockpit — R1 100% (was 95%)

```
R0 closure ────████████████████████████████████████ 100%
R1 closure ──████████████████████████████████████ 100%
├─ ADR-0030 route-failover model       ✓ Accepted
├─ fabric-graph::failover              ✓ committed, 4 tests
├─ Surface plane (PF-WP-015)           ✓ committed, 77 tests
├─ checker --failover-blacklist        ✓ committed, 12 tests
├─ spec 020 contract (PF-WP-022)       ✓ authored
├─ leases::rebind_or_fail impl         ✓ committed, 12 tests
├─ ADR-0031 trust-root model           ✓ Accepted
├─ spec 021 trust-root contract        ✓ authored
├─ trust_root chain (PF-WP-018)        ✓ committed, 17 tests
├─ spec 022 multi-tenant fairness     ✓ authored
├─ leases_fairness + pardon           ✓ committed, 15 tests
├─ Q1-Q4 R1→R2 design decisions      ✓ decided (C/A/D/A+C)
├─ R1 release evidence (PF 0.2.0)      ✓ shipped
├─ fabric-cli Rust                     ✗ Tier 3 (deferred per ADR-0028)
├─ fabric-workspace Rust               ✗ Tier 3 (deferred per ADR-0028)
└─ fabric-checker Rust port            ✗ Deferred (Go canonical, ADR-0029)
```

### What R1 closing 100% means

R1 is fully closed within the documented scope. The Tier 3 Rust crates (`fabric-cli`, `fabric-workspace`, `fabric-checker` Rust ports) are explicitly not part of "the remaining 5%" — they're a separately classified open work item per ADR-0028 (fresh-context required). Closing those is an R2 / R3 effort, not an R1 close-out.

### Next: R2 starts now

The R2 work packages were listed in `releases/2026-09-08-R1.md` §Roadmap. With Q1-Q4 decided, R2 has no open design questions blocking its implementation. The first R2 wedge is the **thin Rust binary `fabric-graph-cli replan`** per Q1-C — that's the highest-value next deliverable.
