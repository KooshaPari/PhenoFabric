# Project Handoff — Phenotype/phenotype-fabric

**For the next session to read on resume.** This document is the
authoritative starting point. The full session history is in
`00_SESSION_OVERVIEW.md` (2,175+ lines), but you only need this
file to resume full project ownership.

**Generated**: 2026-10-15T21:31Z (Round 37)
**Repo**: `github.com/KooshaPari/PhenoFabric`
**Tip at handoff**: `2825bb9`
**Session-commits this arc**: 32 (R1-R36)

---

## TL;DR

The v0.1.2 release pipeline is **operationally complete but
partially shipped**:

- ✓ **GitHub Release v0.1.2 published** with 10 native assets
  (Linux/macOS/Windows GUI installers + CLI tarballs), id 413277409
- ✗ **Docker image not shipped** (bench-stub build failure;
  fix is on main in `d7f280b` but not on the v0.1.2 chain)
- ✗ **Crates not shipped** (needs `CRATES_IO_TOKEN` + 2-of-3
  cargo publish issue)
- ✓ **Dependabot config correct** for Mon 2026-10-20T05:29Z next
  run (R29 / `f8050e7` + R31 / `7648604`)
- ⚠ **4 un-ignored security alerts** (protobuf, jsonwebtoken,
  lru, otel_sdk) — all `patched=null` in GH advisory DB; resolving
  PRs were closed by operator; filter blocks future auto-PRs

**Session is in a stable resting state.** 7 operator-scope items
documented. No agent-actionable work pending.

---

## State snapshot

| Item | Value | Source |
|---|---|---|
| `origin/main` tip | `2825bb9` | R36 |
| Working tree | clean | R36 |
| Open PRs | 0 (all 10 closed by operator 2026-10-15T07:19-07:22Z) | R35 |
| Open issues | 1 (#32, gate to Mon 2026-10-20T05:29Z) | R35 |
| Open alerts | 6 (4 un-ignored + 2 dupes for ignored) | R25 |
| Fixed alerts (this session) | 3 | R25 |
| Production env protection rules | `[]` (unchanged) | R32 |
| Dependabot config | `update-types: [minor, patch]` (R29) + `security-updates-only: true` (R31) | R29/R31 |
| Local branches | 3 (1 main + 2 with unmerged work kept) | R36 |
| Remote branches | 4 (1 main + 3 stale from closed/squash-merged PRs) | R35 |
| Git tags | v0.1.0, v0.1.1, v0.1.2 (all ON-MAIN) | R35 |
| CI on tip | 10/11 pass; only SonarCloud fails (pre-existing) | R33 |
| SonarCloud issues | 10 open (1 MAJOR VULNERABILITY on src-tauri) | R28 CORRECTION |
| v0.1.2 GitHub Release | published, 10 assets, id 413277409, 2026-10-15T04:26:04Z | R34 |

---

## 7 operator-scope items (action menu)

These are all genuinely operator-scope. None have an
agent-actionable path. Pick from the menu.

### 1. CRATES_IO_TOKEN

**Blocker**: publish-crates workflow at `fabric-capability` step
fails for both the missing token AND a 2-of-3 cargo publish issue
documented in R32.

**Agent prep**: workflow code is wired and ready. Only the value
is missing.

**Action**: operator provides the value via web UI
(Settings > Secrets > Actions > New repository secret) or
`gh secret set CRATES_IO_TOKEN`.

### 2. Production env protection rules

**Blocker**: Pro plan API PUT returns 404. Only the POST endpoint
works for new rules on this plan tier.

**Current state**: `protection_rules: []` (id 24340095651,
created 2026-10-15T00:48:16Z).

**Action**: operator configures via web UI
(Settings > Environments > production > Protection rules), OR
upgrades the plan to enable the API, OR accepts current state.

### 3. fabric-frame-transport publishability (R26)

**Current**: 0.1.0 single-crate-rename, treated as 0.0.0
placeholder.

**3 options documented in R26**:
- **A (status quo)**: keep current single-crate rename. No
  further action.
- **B (publish-first)**: publish `fabric-frame-transport` as new
  0.1.0 crate; coordinate with crates.io ecosystem. Code changes
  in the workspace to use the published version.
- **C (git-tag dep)**: use a git-tag-based dep in the workspace
  for now; defer crates.io publish.

**Action**: operator picks A/B/C. Each has compile + test impacts.

### 4. 4 un-ignored alerts (R35)

**Affected**:
- `protobuf` (RUSTSEC-2024-0437, `protobuf 2.27.1` transitive)
- `jsonwebtoken` (RUSTSEC-2024-0436, `jsonwebtoken 10.3.0`)
- `lru` (RUSTSEC-2024-0435, `lru 0.12.5` transitive)
- `opentelemetry_sdk` (RUSTSEC-2024-0429, `opentelemetry_sdk 0.27.1`)

**All have `patched=null`** in the GitHub advisory DB. None can
be auto-resolved by version bumps.

**Resolving PRs were all closed by operator at 07:19-07:22Z**:
- PR #35 opentelemetry-otlp 0.27.0 → 0.33.1 (would resolve
  otel_sdk alert via major-version dep bump)
- PR #37 jsonwebtoken 10.3.0 → 11.1.0 (direct resolve)
- PR #38 ratatui 0.29.0 → 0.30.2 (transitive lru resolve)
- (no resolving PR for protobuf)

**The `update-types: [minor, patch]` filter (R29)** blocks
future auto-PRs for major versions.

**4 paths forward**:
- (a) **Manually bump** those deps + accept the breaking-change
  code work
- (b) **Remove the filter** and re-open PRs #35/#37/#38 (re-add
  the original resolving PRs); handle protobuf separately
- (c) **Wait for upstream patches** (no ETA — `patched=null` is
  the signal)
- (d) **Document as accepted risk**; close alerts as "won't fix"
  with a justification

**Action**: operator picks (a)/(b)/(c)/(d).

### 5. SonarCloud Quality Gate (R28 CORRECTION)

**Pre-existing failure** on `c7367f8` and all 5 prior commits
back to `25d16f4` (R18). Not introduced by R24/R26.

**10 open SonarCloud issues**:
- 1 MAJOR VULNERABILITY: "Dependency versions are not
  predictable" (on src-tauri — protobuf 2.x vs 3.x warning)
- 1 MINOR VULNERABILITY: "clear-text protocols" (probably HTTP
  fetch in src-tauri)
- 8 code smells: `String.fromCharCode`, `replace` vs
  `replaceAll`, regex backtracking, `[[ ]]` vs `[ ]`, label/input
  field, unhandled exceptions

**3 paths forward**:
- (a) **Fix all 10 issues** in src-tauri — involves Tauri Rust
  code, not the main workspace
- (b) **Adjust gate policy** in SonarCloud UI to accept current
  state
- (c) **Accept current state** as a known pre-existing issue;
  do nothing

**Action**: operator picks (a)/(b)/(c). **Do NOT** do a sweep
of unrelated changes to clear the gate (sweep-of-unrelated is
out-of-scope per the agent's standing rule).

### 6. Issue #32 close (R32)

**Open since 2026-10-14**:
"fix(ci): Dependabot security-update run fails with 'No Cargo.toml!'"

**Gate**: the next scheduled dependabot security-update trigger
on **Mon 2026-10-20T05:29Z** must run cleanly (R31 / commit
`7648604` `security-updates-only: true` should silence the
failing job).

**Action**:
1. Next session verifies the Mon run went green (read
   `gh run list --workflow="Dependabot"` for that date)
2. If clean: `gh issue close 32 --comment "Verified Mon
   YYYY-MM-DD run; security-update flow silenced for
   glib + opentelemetry_sdk per commit 7648604."`
3. If still failing: re-investigate, possibly un-ignore the
   deps to see the new error

### 7. 3 stale remote branches (R35)

**Current state**: 3 `origin/*` branches from closed/squash-merged
PRs are still present:
- `origin/feat/global-nested-ledger-20261012` (PR #27 squash-merged)
- `origin/fix/persist-error-exhaustive-and-traceability` (PR #28 closed)
- `origin/fix/spec-validation-infra-gaps` (PR #30 closed)

**Action** (operator-scope per pre_tool hook defers remote branch
deletes):
```bash
git push origin --delete feat/global-nested-ledger-20261012 \
  fix/persist-error-exhaustive-and-traceability \
  fix/spec-validation-infra-gaps
```

---

## Auto-poke pattern (what to expect)

The auto-poke loop fires after ~5min idle. When the operator's
only signal in a turn is `go` or `proc` or a system reminder,
the agent does safe autonomous work:

- Verifies state (`git log`, `gh run list`, `gh issue list`)
- Documents findings in `00_SESSION_OVERVIEW.md`
- Commits with `tx-*` trailers
- Pushes to `origin/main`

**Does NOT** (without explicit operator authorization):
- Force-delete branches (`git branch -D`)
- Push force (`git push --force` / `--force-with-lease`)
- Reset commits (`git reset --hard`)
- Delete the remote repo (`gh repo delete`)
- Open or close issues/PRs
- Delete remote branches
- Modify GitHub secrets or environments
- Run operator's own credentials (CRATES_IO_TOKEN, env rules)
- Make architectural decisions (fabric-frame-transport
  publishability, dependabot top-level config, etc.)

**Signal interpretation**:
- `go` / `proc` → proceed with safe autonomous work
- `do all` → extend to: also do the housekeeping I've been
  deferring
- Specific instruction → execute it literally; ask if blocked

---

## Critical paths for next session (first 5 minutes)

```bash
# 1. Confirm the tip
git log -1 --oneline
# Expected: 2825bb9 docs(session): R36 — local-branch housekeeping ...

# 2. Check CI on tip
gh run list --limit 5
# Expected: 4 most recent success, 1 SonarCloud failure (pre-existing)

# 3. Check open issues
gh issue list --state open
# Expected: 1 issue (#32)

# 4. Check open alerts (optional)
gh api /repos/KooshaPari/PhenoFabric/dependabot/alerts | jq '[.[] | select(.state == "open")] | length'
# Expected: 6

# 5. Check production env (optional)
gh api /repos/KooshaPari/PhenoFabric/environments/production
# Expected: protection_rules: []

# 6. If new commits landed, read R37+ in 00_SESSION_OVERVIEW.md
```

If the Mon 2026-10-20T05:29Z dependabot run has happened:
- Check `gh run list --workflow="Dependabot"` for that date
- If clean: close issue #32 (operator-scope action)
- If failing: re-investigate, possibly un-ignore to see new error

---

## What I would do next (if I were the next session)

In order of expected impact:

1. **Wait for Mon 2026-10-20T05:29Z dependabot run** — verify the
   R31 fix worked; if yes, close #32
2. **Document v0.1.2.x release plan** when operator decides
   between a/b/c on the docker fix forward-port
3. **Triage dependabot config** — should the filter be relaxed to
   allow minor+patch security-updates too?
4. **Prepare v0.1.3** if operator wants to move past the partial
   v0.1.2

---

## Operator preferences (still in force)

These are documented in the main session overview but the
critical ones are:

- **Pre_tool hook defers** (not hard-blocks); `apply_patch` and
  `edit` tools bypass the bash blocklist
- **Don't act on architectural decisions** — wait for explicit
  operator signal
- **Honest framing of scope** — "patched at 0.32.1" on
  `opentelemetry_sdk = "0.27"` is a 5-version minor bump, not a
  patch
- **Auto-poke pattern** → do useful work OR update todos, don't
  bother operator with empty status
- **`[pending.5]` is operator-scope** (public mutation on issue
  tracker)
- **Use `tx-*` trailers** in commits for jcode identity
  (`tx-agent: jcode`, `tx-validated: lint|test|build|cargo-check|manual|none`)
- **Don't fix 10 unrelated code smells** to clear SonarCloud gate
  (sweep-of-unrelated is out-of-scope)

---

## Files in this session folder

- `00_SESSION_OVERVIEW.md` (2,175+ lines) — full round-by-round
  history; read for context, not for resumption
- `HANDOFF.md` (this file) — read FIRST on resume
- `01_RESEARCH.md` — research notes (if any)
- `02_SPECIFICATIONS.md` — specs (if any)
- `03_DAG_WBS.md` — task DAG (if any)
- `04_IMPLEMENTATION_STRATEGY.md` — strategy (if any)
- `05_KNOWN_ISSUES.md` — known issues (if any)
- `06_TESTING_STRATEGY.md` — testing strategy (if any)
- `artifacts/` — diagrams, scripts, etc. (if any)

Only `00_SESSION_OVERVIEW.md` and `HANDOFF.md` are guaranteed to
exist for this session.

---

## Commit ledger (this session, 32 commits)

| Round | Commit | Subject |
|---|---|---|
| R1-R27 | (various) | Initial release-pipeline work |
| R28 (correction) | (various) | SonarCloud check corrected |
| R29 | `f8050e7` | fix(deps): top-level update-types filter to suppress major-version PRs |
| R30 | `6dbb0b4..b5f271c` | 8 dependabot PRs squash-merged |
| R31 | `7648604` | fix(dependabot): silence security-update flow |
| R32-R34 | (various) | Operator-scope items, v0.1.2 release audit |
| R35 | (this R) | Operator closed all 10 PRs; R27 correction |
| R36 | (this R) | Local-branch housekeeping |

---

## What "do all" means in context

The operator's signal was `proc and do all;` followed by
`provid ehandoff for freshc tx tor esume full fproj ownership`.
Interpretation: provide a handoff document so a fresh context
can resume full project ownership.

This HANDOFF.md is the artifact. After committing it, end the
session cleanly. The next session reads this file first.

---

## Handoff contact

- **Session doc**: `docs/sessions/2026-10-15-v012-retag-and-build-debt/00_SESSION_OVERVIEW.md`
- **This handoff**: `docs/sessions/2026-10-15-v012-retag-and-build-debt/HANDOFF.md`
- **Operator**: KooshaPari (koosha@example.com)
- **Repo**: github.com/KooshaPari/PhenoFabric
- **Tip at handoff**: 2825bb9

If anything is unclear on resume, the full session history is
in `00_SESSION_OVERVIEW.md` (2,175+ lines, R1-R36). Read R34
(v0.1.2 release audit) and R35 (operator closed all 10 PRs)
first for current context.
