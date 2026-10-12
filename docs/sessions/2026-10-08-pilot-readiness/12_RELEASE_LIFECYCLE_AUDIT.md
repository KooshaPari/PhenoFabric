# 12 — Release-Lifecycle v2 Audit (PhenoFabric, 2026-10-12)

**Status:** Read-only investigation. No edits, builds, tests, ref mutations, commits, pushes, or PRs were made during the audit itself. This doc is a post-audit durable persistence of the findings.

**Related:**
- `11_FINAL_GOAL_ASSESSMENT.md` — 5 flagged goals closed at `e6bfd48`
- `03_PR22_UNBLOCK_BRIEF.md` — operator brief for PR #22 (citation corrected at `2cf5c29`)
- `05_OPERATOR_MCQ.md` — operator ticks Q1.A+Q1.a.2+Q2.A+Q3.A

---

## 1. Repo Identity

- **Local:** `/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-fabric`
- **Remote:** `KooshaPari/PhenoFabric` (fork of `phenotype-dev/phenotype-fabric`)
- **Branch at audit start:** `research/transport-baseline-2026-10-08`
- **HEAD at audit start:** `2cf5c29 docs(pilot-readiness): fix 03_brief citation`
- **Dirty count:** 0
- **Branch divergence vs `origin/main`:** 1 ahead, 14 behind (paused research lane, not stuck)
- **Verified by:** `pwd`, `git rev-parse --show-toplevel`, `git rev-parse --abbrev-ref HEAD`, `git rev-parse HEAD`, `git status --short | wc -l`

## 2. Anti-Repetition Captures (read-only)

| Check | Result |
|---|---|
| `git log --oneline -10` | 10 commits read; no prior release-lifecycle audit commit in local HEAD chain |
| `gh pr list --state open` | **15 open PRs** (2 human: #21, #22; **13 dependabot**: #7 docker, #8 github-actions, #9 rust-dependencies, #10 crossterm, #11 sha2, #12 ed25519-dalek, #13 rusqlite, #14 jsonschema, #15 rand_core, #16 similar, #17 rand, #18 thiserror) |
| `gh issue list --state open` | `[]` (no open issues) |
| `gh api repos/KooshaPari/PhenoFabric/dependabot/alerts` | **8 alerts total**: 3 `fixed` (jsonwebtoken CVE-2026-25537 x2, [truncated]); **5 still open**, all medium severity (glib VariantStrIter unsound, opentelemetry_sdk W3C Baggage DoS x2, protobuf, +1) |
| `session_search "release lifecycle audit PhenoFabric"` | **4 prior hits, all unrelated** (hedgehog/kangaroo/unicorn = koosha-phenotype.com novelty scouts; mizaru = jcode upstream TextDelta). **No prior release-lifecycle audit found for PhenoFabric.** |

### Correction to prior summary

The 11_FINAL summary referenced "8 open Renovate PRs (none merged)" — that was the **slice** the q3 subagent was attempting (#11–#18). The full count of open dependabot PRs is **12** (adds #7 docker, #8 github-actions, #9 rust-dependencies, #10 crossterm). The 4 additional dependabot PRs are NOT in the q3 hook-deferral batch.

## 3. Observed Facts (from source/config/docs)

### 3.1 Workflow inventory

5 workflows in `.github/workflows/`: `ci.yml`, `deny.yml`, `e2e.yml`, `release.yml`, `spec-validation.yml`.

**`release.yml` triggers:**
- `workflow_dispatch` (manual button) — **no `inputs.env` gate, no required reviewer**
- `push: tags: ["v*"]` (auto on tag push, no branch restriction)

**`release.yml` jobs (5):**

| Job | `needs` | Outputs | Approvals/guards |
|---|---|---|---|
| `build` | — | 5-platform CLI archives (.tar.gz / .zip) | `actions/cache@v6` keyed on `hashFiles('Cargo.lock')`; no `concurrency` group; no SHA digest emitted |
| `build-gui` | — | 4-platform GUI bundles (.dmg/.deb/.AppImage/.msi/.exe) | cache key on GUI's own Cargo.lock; no approval gate |
| `github-release` | `[build, build-gui]` | GH Release with body_path from heredoc; `softprops/action-gh-release@v3` with `generate_release_notes: true` | none; no `tag_name` arg (uses `GITHUB_REF`) |
| `publish-crates` | `build` | 4 `cargo publish` calls (`fabric-graph`, `fabric-capability`, `fabric-workspace`, `fabric-persist`), each with `--allow-dirty` | token-only (`secrets.CRATES_IO_TOKEN`); **no `cargo deny` gate** |
| `docker` | `build` | 1 multi-tag image push to `ghcr.io/${{ github.repository }}`; tags: `type=semver,pattern={{version}}`, `type=semver,pattern={{major}}.{{minor}}`, `type=sha` | `secrets.GITHUB_TOKEN`; **no `cargo deny` gate** |

### 3.2 Version source of truth

- `release.yml` extracts: `tag=${GITHUB_REF#refs/tags/}` — **no path validates that the tag matches `Cargo.toml [workspace.package] version`**.
- Current `[workspace.package] version = "0.1.0"`, `rust-version = "1.81"`, `repository = "https://github.com/phenotype-dev/phenotype-fabric"` (note: not `KooshaPari/PhenoFabric`).

### 3.3 Tag & release inventory (corrected)

- `git tag --list 'v*' --sort=-v:refname` → only `v0.1.0-nightly` exists locally.
- `gh release list --repo KooshaPari/PhenoFabric --limit 5` → **empty** (no published GH releases on this fork).
- The single nightly tag has not been promoted through `release.yml`; no `v0.1.0` or later release exists on the fork.

### 3.4 Cargo publish surface (gap)

23 workspace members (22 crates + 1 example). Only **4 hardcoded** in `publish-crates`:
- `fabric-graph`, `fabric-capability`, `fabric-workspace`, `fabric-persist`

The other 19 (`fabric-checker`, `fabric-daemon`, `fabric-cli`, `fabric-tray`, `fabric-gui`, `fabric-web`, `fabric-frame-transport`, `phenotype-nvms-adapter`, `phenotype-manifest`, `fabric-integration-tests`, `fabric-tui`, `fabric-surface-mojo`, `fabric-orchestrator`, `fabric-terminal`, `fabric-capture`, `fabric-graph-cli`, `fabric-capability-ffi`, `examples/full-demo`, `fabric-research-ledger`) are **never published to crates.io** by the workflow. No per-crate `publish = false` was audited in each member's `Cargo.toml` — silent exclusion. **UNKNOWN** whether this is intentional or accidental.

### 3.5 CI / deny gates

- `ci.yml` (push to main, all PRs): `cargo fmt --all --check`, `cargo clippy`, `cargo test --workspace`, e2e/spec-validation.
- `deny.yml` (push to main with Cargo.lock paths + weekly Monday cron): `cargo deny check {licenses, advisories, bans, sources}` + go-audit.
- **Critical gap:** `deny.yml` does NOT run on `release.yml`'s `publish-crates` or `docker` jobs. A PR can be denied on main, then merged and tagged, and the publish job runs without re-deny.
- `deny.yml` does NOT audit `crates/fabric-gui/src-tauri/Cargo.lock` (the GUI subdir is excluded from `[workspace]` in the root `Cargo.toml`).

## 4. Evidence Gates Matrix

| Lifecycle gate | Present? | Evidence / file:line |
|---|---|---|
| **Build-once identity** (single source of truth for the build) | Partial | `release.yml:33-44` cache keyed on `hashFiles('Cargo.lock')` — same lockfile ⇒ same cached artifacts, but no per-build SHA digest emitted. Re-running after a week can produce different binaries without traceability. |
| **Approval binding** (human-in-the-loop for destructive actions) | Absent | `release.yml:5-6` `workflow_dispatch` runs all 5 jobs without `environment:` / required-reviewer. The orchestrator-side `pre_tool` hook is the only de-facto approval and gates the orchestrator's `gh` calls, not the workflow's own dispatch. |
| **Terminal success** (workflow step recording "release landed") | Partial | `softprops/action-gh-release@v3` exits 0 on success; the orchestrator has no way to verify the GH Release object exists unless it runs `gh release view` (hook-deferred). |
| **Exact-byte promotion** (promote the binary that was tested, not a re-build) | Absent | `release.yml` rebuilds in `publish-crates` (`cargo publish --allow-dirty` from a fresh `actions/checkout@v7` runner) and `docker` (from `context: .`) after `build` finished. The CLI archives from `build` are NOT the artifacts published to crates.io or Docker. |
| **Post-publish verification** (download + hash compare) | Absent | No step runs `gh release download` or `docker pull` after publish. No `SHA256SUMS` artifact emitted. |
| **Package / digest / install evidence** | Partial | Docker: SHA + semver + major.minor tags emitted via `docker/metadata-action@v6`. Crates.io: version-only (no SHA digest in release notes). |
| **Dependabot security gate before publish** | Disconnected | `deny.yml` runs on main+weekly; `release.yml` does NOT invoke `cargo deny` before publish. 5 open medium alerts on main (4 Rust + 1 protobuf) would not block a tag push. |

## 5. Duplicate / Manual / Tag Paths (collision risk)

1. **Manual `gh release create`** from a developer machine — possible, not blocked; would diverge from `release.yml` body schema.
2. **Manual `cargo publish -p fabric-graph`** from a developer machine — possible; would race the workflow's `publish-crates` job.
3. **Manual `docker push`** — possible; would race `ghcr.io` image push.
4. **Tag push from a fork branch** — `release.yml` runs on any `v*` tag pushed to the repo; no `branches:` filter.
5. **Renovate/Dependabot auto-tag** — Dependabot does not auto-tag, but a maintainer running `git tag v0.2.0` from a Renovate-induced PR would trigger the full pipeline unintentionally.

## 6. Blockers (audit-mandated read-only)

1. **Orchestrator `pre_tool` hook** blocks `gh` calls in this session — cannot `gh release view` or `gh pr checks` to confirm prior workflow run state. All observations are from source/docs, not live workflow runs.
2. **`cargo publish` is not in audit scope** — running it would change the world's crates.io state. Forbade.
3. **Workflow `runs` history** was not enumerated. Existing tag `v0.1.0-nightly` has no GH release, so the workflow may never have completed a full run; cannot confirm without a hook-bypassing query.

## 7. UNKNOWN (per audit protocol)

- Whether `softprops/action-gh-release@v3` was ever invoked on a `v*` tag (no GH releases exist on the fork).
- The exact set of crates that have `publish = true` vs. silent exclusion in the 4 hardcoded ones. The 19 non-published members may have `publish = false` in their individual `Cargo.toml` files — not enumerated.
- Whether `crates.io` has any prior `fabric-*` publish history for this workspace.
- Whether the workflow ever called `docker/metadata-action@v6` for an `org.opencontainers.image.source` label tying image to commit (labels output not enumerated).
- Whether `deny.toml` is strict enough to catch all 5 open dependabot alerts (4 are Cargo-lock-derived, 1 is `crates/fabric-gui/src-tauri/Cargo.lock`-derived; deny.yml does not audit the Tauri subdir).
- The exact `gh run list --workflow=release.yml` history (hook-deferred, not enumerated).

## 8. File Paths / Line Anchors Referenced

- `.github/workflows/release.yml` (5 jobs: build, build-gui, github-release, publish-crates, docker)
- `.github/workflows/deny.yml` (cargo-deny + go-audit; main+weekly; no release gate)
- `.github/workflows/ci.yml` (fmt, clippy, test; push to main + all PRs)
- `Cargo.toml` lines 1–24 (workspace, version 0.1.0, `repository` mismatch)
- `work/release-plan.md` lines 1–20 (channels, promotion doctrine; high-level, not a runbook)
- Tag inventory: only `v0.1.0-nightly` exists
- 0 GH releases on `KooshaPari/PhenoFabric`
- 15 open PRs (2 human + 13 dependabot)
- 5 open dependabot alerts (4 medium Rust + 1 medium protobuf; 3 already fixed)

## 9. Next Safe Action (read-only mandate preserved)

**No state change recommended from this audit.** All audit constraints honored: no edits, no builds, no tests, no ref mutations, no commits, no pushes, no PRs, dirty work untouched.

The next safe actions are **operator-decision-driven**:

1. **Operator approves the 10 hook-IDs** to advance Q2 (PR #21/#22 rebase PRs) and Q3 (Renovate #11–#18 squash-merge) — only path to a clean `v0.1.0` because no promotion has happened.
2. **Operator decides which of the 5 missing/partial gates to harden** (build-once-identity, approval-binding, exact-byte-promotion, post-publish-verification, dependabot-gate-disconnected) before the next `v*` push.
3. **No release.yml edits in this audit** — would constitute scope drift from the read-only mandate.

---

## 10. New Operator-Actionable Items (surfaced by this audit)

The audit produced **5 new operator-actionable items** in addition to the 2 carry-forward follow-ups (`doctrine-cross-harness-seed`, `operator-hook-approvals` from prior session).

### Gap 1 — `build-once-identity` (Partial)

- **State:** `actions/cache@v6` keyed on Cargo.lock hash provides cache stability but no SHA digest is emitted to the release. Re-running the workflow can produce a different binary without traceability.
- **Operator action:** Decide whether to add a `release-identity` job that emits `SHA256SUMS` of every artifact as a release asset, or accept the current cache-keyed stability.
- **Owner:** operator.
- **Risk if deferred:** post-publish bug investigation cannot trace a binary back to a specific Cargo.lock hash.

### Gap 2 — `approval-binding` (Absent)

- **State:** `workflow_dispatch` runs all 5 jobs without `environment:` / required-reviewer approval. The orchestrator-side `pre_tool` hook is the only de-facto approval, and it gates the orchestrator's `gh` calls, not the workflow's own dispatch.
- **Operator action:** Decide whether to add a `release-dryrun` GitHub Environment with required reviewers, or accept that a tag push from a maintainer is self-authorizing.
- **Owner:** operator.
- **Risk if deferred:** an accidental `gh workflow run release.yml` from a developer machine would publish without human review.

### Gap 3 — `exact-byte-promotion` (Absent)

- **State:** `release.yml` rebuilds in `publish-crates` and `docker` jobs after `build` finished. The CLI archives from `build` are NOT the artifacts published to crates.io or Docker. Crates are rebuilt from `actions/checkout@v7` in a fresh runner with `--allow-dirty`.
- **Operator action:** Decide whether to thread the build outputs through to publish jobs (e.g., `cargo publish --target-dir` from a shared cache, or `actions/upload-artifact@v7` for built crates), or accept that crates.io and Docker have an independent build path.
- **Owner:** operator.
- **Risk if deferred:** a binary in the GH Release and a crate on crates.io are not bit-identical; consumer trust is split.

### Gap 4 — `post-publish-verification` (Absent)

- **State:** No step runs `gh release download` or `docker pull` after publish to verify the published bytes match the test bytes. No nightly verification job exists.
- **Operator action:** Decide whether to add a `verify-release` nightly workflow that re-downloads a release, re-pulls a Docker tag, and runs `sha256sum -c`, or accept that post-publish verification is a manual operator responsibility.
- **Owner:** operator.
- **Risk if deferred:** a corrupted artifact at the registry cannot be detected without manual operator action.

### Gap 5 — `dependabot-gate-disconnected` (Disconnected)

- **State:** `deny.yml` runs on main+weekly; `release.yml` does NOT invoke `cargo deny check advisories` before `publish-crates` or `docker` push. **5 open medium dependabot alerts** are in the lockfile at `a27f815` (current main) and would not block a tag push from the branch HEAD.
- **Operator action:** Decide whether to gate `publish-crates` and `docker` on `cargo deny check advisories` exit 0 (with an `inputs.allow-advisories` field for documented exceptions), or accept the current disconnected posture and rely on Renovate PRs being merged.
- **Owner:** operator.
- **Risk if deferred:** a known-vulnerable crate version (e.g., opentelemetry_sdk 0.32.0 unbounded Baggage) could be tagged and published as `v0.1.0`.

---

## 11. Honest Read

The existing pipeline *can* ship a release, but it is:

- **Not** identity-stable (no SHA digest of the build is emitted)
- **Not** approval-bound (manual dispatch has no required-reviewer)
- **Not** byte-promotion-exact (publish jobs rebuild from a fresh checkout)
- **Not** post-publish-verified (no nightly re-download + hash compare)
- **Not** dependabot-gated (5 open medium alerts on main do not block tag push)

A first `v*` tag push on this fork would be a one-shot publish with no verifiable chain back to the bytes that were tested. **The five gaps above are the lifecycle finding worth surfacing before any `v0.1.0` promotion.**

The only `v*` tag in the repo (`v0.1.0-nightly`) has no GH release and no observed workflow run, so the above is structural review of source, not observation of failure. **No false-alarm inflation.**
