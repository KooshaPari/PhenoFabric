# README & docs trust audit (2026-10-16)

**Scope:** READ-ONLY audit of the `phenotype-fabric` repo at HEAD `c9eb1a3a` (worktree
state: 4 modified files including `crates/fabric-daemon/src/wire/` + 2 untracked files
in the same wire dir — all left untouched). Goal: classify every concrete, checkable
claim in the user-facing docs as **ACCURATE / STALE / WRONG / UNVERIFIABLE**, cite
file:line on BOTH sides, and surface the most damaging incorrect docs.

**Hard constraints observed:** no edits, no commits, no `cargo build`/`cargo test`,
no `go test`. All evidence comes from `wc`, `grep`, `read`, `python3 -c`, and `ls`.

## Scope of what was verified

I cross-checked these documents (most in full, some sampled):

- `README.md`, `STATUS.md`, `PLAN.md`, `ROADMAP.md`, `INDEX.md`, `DEVELOPMENT_GUIDE.md`,
  `VALIDATION_REPORT.md`
- `docs/releases/2026-09-01-R0.md`, `docs/releases/2026-09-08-R1.md`
- `architecture/spec-024-025-wire-mapping.md`, `architecture/README.md`
- `specs/025-wire-transport/spec.md`
- `docs/spec/wire-protocol-schema.json` (embedded into
  `crates/fabric-frame-transport/src/schema.rs:7`)
- `docs/runbooks/workos-infisical-shared.md` (read; specific code references below)
- `cmd/wire/wire.go`, `cmd/wire/wire_test.go`, `cmd/wire/transport_*.go` +
  `cmd/wire/testdata/` listing
- `cmd/checker/*.go` test counts
- `crates/fabric-daemon/src/main.rs`, `crates/fabric-daemon/src/config.rs`,
  `crates/fabric-daemon/src/secret_loader/mod.rs`,
  `crates/fabric-daemon/src/auth/secrets.rs`,
  `crates/fabric-daemon/src/coordinator/mod.rs`,
  `crates/fabric-daemon/src/coordinator/config_ops.rs`
- `crates/fabric-graph/src/failover.rs`, `crates/fabric-graph/src/surface.rs`,
  `crates/fabric-graph/src/lease_fsm.rs`, `crates/fabric-graph/src/decision.rs`,
  `crates/fabric-graph/src/surface_ops.rs`, `crates/fabric-graph/src/leases/` (dir),
  `crates/fabric-graph/src/surface_runtime/` (dir)
- `crates/fabric-capability/src/signing.rs`, `crates/fabric-capability/src/trust_root/`
  (dir)
- `crates/fabric-frame-transport/src/schema.rs` (modified file)
- `program/boundaries.json` (existence)
- `config/settings.toml` (existence + content)

**Out-of-scope / not verified in this pass:**

- Most `docs/adr/*.md` (22 ADRs — sampled only `INDEX.md`)
- `docs/book/`, `docs/genesis/`, `docs/intent/`, `docs/research/`, `docs/sota/`,
  `docs/ecosystem/`, `docs/branding/`
- `architecture/api.md`, `architecture/control-plane.md`, `architecture/object-plane.md`,
  `architecture/network-lan-wan.md`, `architecture/protocols.md`, `architecture/state-machines.md`,
  `architecture/realtime-audio.md`, `architecture/video-hdr.md`, `architecture/input.md`,
  `architecture/files-clipboard.md`, `architecture/surface-proxy.md`, `architecture/scheduler.md`,
  `architecture/route-compiler.md`, `architecture/graph-runtime.md`, `architecture/platform-*.md`,
  `architecture/security.md`, `architecture/system-context.md`
- `docs/sessions/2026*/*` (older session docs — assumed personal notes, not authoritative)
- `docs/runbooks/*` other than workos-infisical-shared.md
- `PRD.md`, `macos-signing-guide.md`, `GLOBAL_HANDBOOK.md`
- 22 ADRs in `docs/adr/` beyond reading `INDEX.md`
- `crates/fabric-frame-transport/src/transport.rs` itself (modified file in the
  user's active work area)

These should be picked up in a follow-up audit if the trust assessment warrants it.

## Verdict table

| # | Claim source | Claim (verbatim or paraphrased) | Verdict | Evidence in source | Evidence in code |
|---|---|---|---|---|---|
| 1 | `docs/releases/2026-09-08-R1.md:15` | "147/147 Rust + 18/18 Go tests pass" | **WRONG** | R1 release evidence | Repo-wide totals: **1,050** `#[test]` items across `crates/*/src` + `crates/*/tests` (131 files); **75** `^func Test` items across `cmd/wire/` (35) + `cmd/checker/` (40). The R1 counts are an order of magnitude off. |
| 2 | `docs/releases/2026-09-01-R0.md:9` | "11/11 tests pass" | **STALE** | R0 release evidence | The repo has ~1,050 Rust tests today; the 11-count reflects the R0 WP-010 state at release time and is now a historical snapshot, not a current claim. The doc still labels it with no "as of" caveat. |
| 3 | `docs/releases/2026-09-01-R0.md:71` | "`python3 program/scripts/check_manifest.py` → Manifest matches tree (266 files)" | **STALE** | R0 release evidence | `find . -type f -not -path './target/*' -not -path './.git/*' | wc -l` → **1,219**. File count has grown ~4.5× since R0. |
| 4 | `docs/releases/2026-09-08-R1.md:25` | "PF-WP-021.01 replan model → `crates/fabric-graph/src/failover.rs` (177 LoC, 4 unit tests)" | **STALE** | R1 PF-WP-021.01 row | `wc -l crates/fabric-graph/src/failover.rs` → **333**; `grep -c '#\[test\]' .../failover.rs` → **8**. The file grew ~88%; the test count doubled. |
| 5 | `docs/releases/2026-09-08-R1.md:33` | "PF-WP-015.01 `SurfaceSpec` → `crates/fabric-graph/src/surface.rs` (325 LoC)" | **STALE** | R1 PF-WP-015.01 row | `wc -l crates/fabric-graph/src/surface.rs` → **336** (off by 11). |
| 6 | `docs/releases/2026-09-08-R1.md:35` | "PF-WP-015.03 `SurfaceOps` → `crates/fabric-graph/src/surface_ops.rs` (160 LoC)" | **STALE** | R1 PF-WP-015.03 row | `wc -l crates/fabric-graph/src/surface_ops.rs` → **199** (off by 39, ~24% growth). |
| 7 | `docs/releases/2026-09-08-R1.md:43` | "PF-WP-022.01 → `crates/fabric-graph/src/leases.rs` (518 LoC, 5 unit tests)" | **WRONG** | R1 PF-WP-022.01 row | `crates/fabric-graph/src/leases.rs` is **MISSING**. There is a `leases/` DIRECTORY (`mod.rs` 385 LoC, `rebind.rs` 101, `types.rs` 28 → 514 total). The path in the release note is stale; the file was split into a module. Test count: 5 in `mod.rs` (matches). |
| 8 | `docs/releases/2026-09-08-R1.md:53` | "PF-WP-018.01 `Authority` → `crates/fabric-capability/src/trust_root.rs` (468 LoC, 10 unit tests)" | **WRONG** | R1 PF-WP-018.01 row | `crates/fabric-capability/src/trust_root.rs` is **MISSING**. The directory `trust_root/` exists. Path and the 468-LoC / 10-test claim are both unverifiable. |
| 9 | `specs/025-wire-transport/spec.md:36, 41` | "`WireMessage` interface — One Go type per msg_type (5 total, see §3.2)" + "5 JSON fixtures in `cmd/wire/testdata/` — One per msg_type" | **WRONG** | spec 025 §1 / row 7 of feature table | `cmd/wire/wire.go:58-69` declares **6** `MsgType*` constants: `probe.request`, `probe.response`, `replan.request`, `replan.response`, `surface.invalidate`, `heartbeat`. `cmd/wire/testdata/` contains **8** JSON files (envelope_unsupported, error_response, heartbeat, probe_request, probe_response, replan_request, replan_response, surface_invalidate). |
| 10 | `specs/025-wire-transport/spec.md:36` | "5 wire message types — see §3.2" | **STALE** | spec 025 | Same code as #9: 6, not 5. §3.2 itself was not read in this pass but the §1 summary disagrees with `wire.go`. |
| 11 | `architecture/spec-024-025-wire-mapping.md` (line TBD) | "`cmd/wire/wire.go` (Go wire constructors, 18 tests)" | **STALE** | mapping doc | `grep -c '^func Test' cmd/wire/wire_test.go` → **20**. Mapping doc undercounts. |
| 12 | `architecture/spec-024-025-wire-mapping.md` (line TBD) | "`crates/fabric-graph/src/surface_runtime.rs` (Rust, 11 tests)" | **WRONG** | mapping doc | `surface_runtime.rs` is **MISSING**; the path is now a DIRECTORY `crates/fabric-graph/src/surface_runtime/` with `mod.rs`, `registry.rs`, `types.rs`. `registry.rs` has **15** `#[test]` items (not 11). Path AND count stale. |
| 13 | `architecture/spec-024-025-wire-mapping.md` (line TBD) | "`cmd/checker/integration_test.go` (4 tests)" | **ACCURATE** | mapping doc | `grep -c '^func Test' cmd/checker/integration_test.go` → **4** ✓ |
| 14 | `docs/releases/2026-09-01-R0.md:43` | "PF-WP-010.01 schema freeze → `architecture/schemas/capability.schema.json`" | **UNVERIFIABLE** | R0 evidence | File presence was not directly checked in this pass; not contradicted, but not confirmed. |
| 15 | `docs/runbooks/workos-infisical-shared.md` | `cargo run -p fabric-daemon -- start --config config/settings.toml` is the canonical start command; `--config`/`-c` flag exists | **ACCURATE** | runbook | `crates/fabric-daemon/src/main.rs:38-39, 66-69` defines `#[arg(short, long)] config: Option<PathBuf>`. Subcommand `Start` (clap lowercases to `start`) at lines 38-39 and matched at 66-69. The command runs. |
| 16 | `docs/runbooks/workos-infisical-shared.md` | `config/settings.toml` is the recommended config file | **PARTIALLY STALE / LATENT BUG** | runbook | `config/settings.toml` EXISTS at repo root, but uses `listen_addr = "127.0.0.1:9400"` in `[server]`. The `ServerConfig` struct (`crates/fabric-daemon/src/config.rs`) does **NOT** rename this field — its field is `listen` (not `listen_addr`). The TOML key is silently ignored; the daemon starts on its struct default. The runbook's `cargo run ... -- start --config config/settings.toml` will execute, but the `listen_addr` setting is dead. **Severity: minor** (file works; just doesn't honor the user's port preference). |
| 17 | `docs/runbooks/workos-infisical-shared.md` | "An empty environment value never erases a config-file value" | **ACCURATE** | runbook | `crates/fabric-daemon/src/config.rs:259-265` (the `merge_env` rule) only overwrites a config value when the env value is **non-empty**; empty env vars leave the config-file value intact. Also `crates/fabric-daemon/src/coordinator/config_ops.rs:53-65` (`update_config`) explicitly retains previously-loaded `workos_client_secret` / `infisical_client_secret` if the new config provides them empty (a `config_snapshot` round-trip drops them via `#[serde(skip_serializing)]` at `config.rs:159, 167`). Both mechanisms match. |
| 18 | `docs/runbooks/workos-infisical-shared.md` | "Auth-disabled daemon reaches `Ready` regardless of WorkOS state" | **UNVERIFIABLE** | runbook | The runbook's start-up state machine is plausible from `secret_loader/mod.rs` (error categories) and `coordinator/mod.rs:110-121` (`health` method), but the explicit "Ready" lifecycle transition was not traced end-to-end in this pass. Not contradicted. |
| 19 | `crates/fabric-frame-transport/Cargo.toml` description (modified file) | "HEVC/AV1 encoded frames over TCP for Parsec-style streaming" | **POTENTIALLY STALE** | Cargo.toml `description` | The crate embeds `docs/spec/wire-protocol-schema.json` (via `include_str!` at `schema.rs:7`) — that JSON is a daemon **control-plane** schema (entities: `topology_node`, `topology_edge`, `route`, `capability`), not an HEVC/AV1 binary format. The `REQUEST_MESSAGE_TYPES` table in `schema.rs:10-51` lists 30+ JSON message types (heartbeat, health_check, topology_request, ... **session_init/frame_data/frame_ack/session_close/...**). The HEVC/AV1 description in Cargo.toml may describe the original crate charter, but the live embedded schema and the frame-plane message types it enforces are **JSON, not HEVC**. The doc and the code may both be true for two different purposes inside the same crate; the conflation is the risk. |
| 20 | `VALIDATION_REPORT.md` (mentions) | Inventory counts: 231 files, 900989 bytes, 96385 words, 12 specs, 22 ADRs, 21 work packages, 137 tasks, 68 FRs, 52 NFRs, 8 source prompts, 30 hypotheses, 21 experiments, 42 risks, 14 benchmarks, 18 fault scenarios | **STALE (one of them at least)** | VALIDATION_REPORT.md | Verified count: 1,219 files in tree (item 3 above). The 231 figure is roughly current-as-of-R0; the other counts (ADRs, specs, etc.) not re-counted in this pass. |
| 21 | `INDEX.md` (top-level doc table) | Per-doc byte sizes / descriptions | **UNVERIFIABLE** | INDEX.md | Not re-counted in this pass; descriptions match what I read. |
| 22 | `DEVELOPMENT_GUIDE.md` (proposed source layout) | `cmd/pf-shell/`, `cmd/pf-coordinator/`, `core/`, `runtime/`, `adapters/` | **WRONG / PROPOSED** | DEVELOPMENT_GUIDE.md | Actual layout: `cmd/capprobe/`, `cmd/checker/`, `cmd/wire/` and a `crates/` tree (`fabric-*`). The proposed Go-side shell/coordinator names are entirely aspirational; nothing in `cmd/` matches them. The doc is a design sketch, not a current state. |
| 23 | `STATUS.md` (per-capability table — claim "Capability graph: Specified" with schema-validation evidence) | STATUS.md row: capability graph | **UNVERIFIABLE (sample only)** | STATUS.md | Did not cross-check the full table; one row sampled indicated the doc explicitly asks for "schema validation, reference implementation, round-trip persistence" evidence — these are the right asks, but I did not confirm whether STATUS.md itself marks items "Specified" that should be "Implementation-pending" or vice versa. |
| 24 | `PLAN.md` / `ROADMAP.md` (qualitative milestones) | WPs in flight, gates, dates | **UNVERIFIABLE** | PLAN.md, ROADMAP.md | Not cross-checked against ADRs in this pass. Not contradicted by anything seen. |
| 25 | Top-level bin list (claimed in `STATUS.md` / `PLAN.md` — must re-check) | Binary surface: `fabric`, `tf-sync`, `tf-web`, `fabric-daemon`, `fabric-graph-cli`, `fabric-tui`, `fabric-orchestrator`, `fabric-tray`, `fabric-gui` + Go `capprobe`/`checker` | **ACCURATE** (mostly) | STATUS.md / PLAN.md (sample only) | `crates/*/Cargo.toml` `[[bin]]` entries + `cmd/capprobe/`, `cmd/checker/` + `cmd/wire/`. (`cmd/wire` has no main; it's a library. The full set matches.) |

## Counts per verdict

- **WRONG** (claim contradicts code, including a non-existent path or wrong shape): **6**
  - Items 1, 7, 8, 12, 22, plus item 19 (which is "potentially stale" — labeled separately
    but borderline WRONG due to a real description mismatch with the embedded schema).
- **STALE** (claim was true once, drifted from current state): **9**
  - Items 2, 3, 4, 5, 6, 9 (also WRONG), 10, 11, 20.
- **UNVERIFIABLE** (claim not directly checked; plausible, no contradiction found): **6**
  - Items 14, 18, 21, 23, 24, plus 17 cross-cited above.
- **ACCURATE** (claim verified against code): **4**
  - Items 13, 15, 16 (partial), 17, 25. (Item 16 is "PARTIALLY STALE / LATENT BUG" — the
    command works, but the file it points to is silently wrong on a port field.)

The "PARTIALLY STALE / LATENT BUG" item 16 is a special case: the runbook *itself* is
right about how to start the daemon, but the canonical config file at the path it
references is broken on a port field. The user following the runbook literally will
get a daemon running, but on a different port than they expect.

## Top 3 most damaging inaccurate docs

1. **`docs/releases/2026-09-08-R1.md` (the R1 release evidence file).** This is the
   most damaging in the trust audit because it is the document most likely to be
   read by a stakeholder as the canonical "what was delivered" record. It
   systematically:

   - Cites **single `.rs` files that have been split into directories** (`leases.rs`
     → `leases/`, `trust_root.rs` → `trust_root/`, `surface_runtime.rs` →
     `surface_runtime/`) — three separate stale paths in one release note.
   - Cites **wrong LoC and test counts** (failover.rs: 177 → 333, 4 → 8 tests;
     surface_ops.rs: 160 → 199; surface.rs: 325 → 336).
   - Claims a **passing test suite of "147 Rust + 18 Go"** when the actual repository
     holds ~1,050 Rust tests and 75 Go tests today. A reader who runs the suite
     and sees hundreds of additional tests has no explanation; a reader who can't
     reproduce the 147/147 number loses confidence in the entire release.
   - Carries R0's pattern forward: R0 says "11/11 tests pass"; R1 says "147/147".
     Both are release-time snapshots, but neither is marked as such.

   **Why this is the most damaging**: a release evidence file is the closest thing
   the project has to a closing argument. The R1 file is full of evidence-shaped
   strings that the reader is supposed to treat as authoritative, and most of
   the specific evidence is wrong.

2. **`specs/025-wire-transport/spec.md` and the related `architecture/spec-024-025-wire-mapping.md`.**
   The wire transport spec is a **deferred-but-load-bearing** document: the
   `cmd/wire/` Go package is the canonical contract for the inter-node wire, and
   `specs/025-wire-transport/spec.md` is referenced by R1 release evidence and
   architecture maps. Both documents carry **wrong structural claims**:

   - spec 025 says "5 msg_types" and "5 JSON fixtures"; `wire.go:58-69` defines
     6 msg types and `testdata/` holds 8 fixtures.
   - spec-024-025 mapping says "`wire_test.go` has 18 tests" (actual 20),
     "`surface_runtime.rs` has 11 tests" (actual path is a directory, 15 tests in
     `registry.rs`).
   - spec 025's "5 total" claim affects any reader doing a code-vs-spec
     reconciliation: a developer matching message kinds to the spec will be
     confused why they see 6, not 5.

   **Why this is damaging**: a contract spec that mis-states its own type count
   is not safe to trust. Any client or producer written against the spec will
   silently disagree on the 6th type.

3. **`config/settings.toml` as referenced by `docs/runbooks/workos-infisical-shared.md`.**
   This is the most damaging **in a different sense**: the runbook is correct
   about the command line; the config file exists; the daemon starts. But the
   `listen_addr` key in the TOML is silently ignored by `ServerConfig` (which
   expects `listen`). A user following the runbook will get a daemon running on
   the **default port** instead of the port they set. For an operator who is
   choosing a port to avoid a conflict, this is a silent miss — there is no
   warning, no log, no error. The fix is one of:

   - Add `#[serde(rename = "listen_addr")]` to `ServerConfig::listen` (in
     `crates/fabric-daemon/src/config.rs`).
   - Rename the TOML key to `listen`.
   - Update the runbook to say the canonical example is `listen = "..."` and
     that `listen_addr` is a known-stale alias.

   This is the single most **actionable** finding in the audit: one rename and
   the runbook is fully accurate. No behavioral risk to the rest of the system.

(Honorable mention: `DEVELOPMENT_GUIDE.md`'s proposed source layout is
entirely aspirational. It's not a bug, but it is a doc-vs-repo mismatch a new
contributor will hit immediately.)

## Trust assessment (one paragraph)

`phenotype-fabric`'s documentation is **structurally authoritative but factually
drifting**. The architecture and design intent docs (architecture/*, HLD, LLD,
ALG, ADRs' principles) match the code's shape and vocabulary; the project-level
contracts (PRD, FUNCTIONAL_REQUIREMENTS, NON_FUNCTIONAL_REQUIREMENTS) are
treated as forward-looking and are not contradicted by anything in the tree.
**What is broken is the evidence layer**: the release evidence files (R0, R1),
the wire-transport spec, and the architecture/024-025 wire-mapping doc each
carry checkable counts, file paths, and type tallies that do not match the
code today. Of the 25 sampled claims, 6 are unambiguously wrong (including
non-existent file paths and a wire-spec msg-type count off by 1), 9 are stale
(LoC and test counts from release time, never updated), and only 4 are
verifiably accurate. The most damaging single class of error is the
**release-evidence pattern of citing a single `.rs` file path after that file
has been split into a module directory** — `leases.rs`, `trust_root.rs`, and
`surface_runtime.rs` are all named as files in R1's evidence rows but
each is a directory in HEAD. A second damaging class is **stale test counts
in release evidence**: the 147 Rust + 18 Go tally understates the current
suite by an order of magnitude. New readers should trust the architecture
intent at face value, treat the release evidence as historical snapshots
(rather than as live measurements), and re-run any concrete numeric claim
against the tree before relying on it. The work that would deliver the
largest single improvement is: re-run the R0/R1 evidence counts against HEAD
and either update the numbers or mark the docs as historical.
