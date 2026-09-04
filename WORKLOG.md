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
