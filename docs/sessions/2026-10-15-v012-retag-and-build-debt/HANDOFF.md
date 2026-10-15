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
- ⚠ **4 open security alerts** — `glib` x2, `opentelemetry_sdk`
  x2, both with patched versions available (R38 correction; an
  earlier draft said 6 including protobuf/jsonwebtoken/lru, which
  is wrong — those are already fixed in the lockfile)
- ⚠️ **cargo-deny pin (R38, attribution corrected).** The
  dependency-audit gate was failing on a `deny.toml` schema
  mismatch; commit `33164d7` already fixed that on 2026-10-14 and
  it has been green since. R38 additionally pinned cargo-deny to
  0.20.2 for determinism — good hygiene, but **not** the fix, and
  an earlier R38 claim that it was the fix was wrong. Verified
  green on ubuntu-latest via run `38811471799`. See item 7.
- 🔴 **Issue #32 is STILL BROKEN (R38, live evidence).** Not a
  Monday-cron question — ad-hoc Dependabot security-update jobs
  rooted at `/crates/fabric-daemon` and
  `/crates/fabric-gui/src-tauri` are failing **right now** with the
  original `No Cargo.toml!` error. 7 of 7 such jobs failed in the
  last 24h, while the sibling job rooted at `/` passes. R31's
  `ignore` entries did not suppress them. **Do not close #32.**
  See item 6.

**Session is in a stable resting state.** 9 operator-scope items
documented (7 original + 2 from R38). Three of the R37 handoff's
claims were materially wrong and are corrected here: the alert set
(4 open, not 6), the issue-#32 close gate (would have authorised a
false close, and the bug is live right now), and the ratatui
alert (never existed). R38 also had to correct itself twice —
adversarial review is recorded in item 7.

---

## State snapshot

| Item | Value | Source |
|---|---|---|
| `origin/main` tip | `2825bb9` | R36 |
| Working tree | clean | R36 |
| Open PRs | 0 (all 10 closed by operator 2026-10-15T07:19-07:22Z) | R35 |
| Open issues | 1 (#32, gate to Mon 2026-10-20T05:29Z) | R35 |
| Open alerts | **2** (R40; was 4) — `glib` #1, #8 only. Both proven unactionable: glib 0.20 unreachable, and the vulnerable `VariantStrIter` API is never called here | R40 |
| Fixed alerts (this session) | 3 | R25 |
| Production env protection rules | `[]` (unchanged) | R32 |
| Dependabot config | `directories: ["/", "/crates/fabric-gui/src-tauri"]` (R39 restructure; R31 `security-updates-only` ignores removed as disproven) | R39 |
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

### 4. 4 open security alerts (CORRECTED R38)

**CORRECTION (R38)**: the earlier draft of this handoff listed 6
open alerts (`protobuf`, `jsonwebtoken`, `lru`, `opentelemetry_sdk`)
and stated all had `patched=null`. That is **wrong**. Re-verified
against the live API on 2026-10-15T21:34Z:

**Actually 4 open alerts, 2 packages, BOTH with a patched version:**

| # | Package | Sev | Patched in | Status |
|---|---|---|---|---|
| 1 | `glib` | medium | `0.20.0` | lockfile at `0.18.5` |
| 8 | `glib` | medium | `0.20.0` | lockfile at `0.18.5` |
| 4 | `opentelemetry_sdk` | medium | `0.32.1` | **CLOSED in R40** — crates deleted |
| 5 | `opentelemetry_sdk` | medium | `0.32.1` | **CLOSED in R40** — crates deleted |

`protobuf`, `jsonwebtoken` and `lru` alerts are **already closed** —
the lockfile confirms the resolving bumps landed:
`jsonwebtoken 11.1.0`, `protobuf 3.7.2`, `lru 0.18.5`. The
`update-types: [minor, patch]` filter does **not** block a security
advisory that already has a patched version, so the R29 filter is
not the constraint it was described as being.

**R38 correction (from adversarial review):** R38 also claimed
`ratatui` alerts were closed. **That is wrong — no ratatui alert
has ever existed**; `dependabot/alerts?state=all` returns no rows
for it. ratatui 0.30.2 is simply an upgrade. The actual closed set
is **5** alerts, which R38 also undercounted by omitting one:

| Alert | Package | Patched in |
|---|---|---|
| #9 | `rustls` | 0.23.45 |
| #7 | `jsonwebtoken` | 10.3.0 |
| #6 | `jsonwebtoken` | 10.3.0 |
| #3 | `protobuf` | 3.7.2 |
| #2 | `lru` | 0.16.3 |

Note `lru` sits at 0.18.5 against a 0.16.3 fix threshold, so it is
genuinely fixed, not papered over. Meanwhile `glib` at 0.18.5
against a 0.20.0 threshold is exactly why #1/#8 are legitimately
still open.

`opentelemetry_sdk` was the sharp edge: two copies coexisted
(`0.27.1` vulnerable, `0.33.1`). Whichever crate still pinned
`0.27.1` was the one that needed the bump.

#### R40: RESOLVED — otel deps deleted, alerts #4 and #5 auto-closed

Operator said `go`; the two otel alerts were the only remaining
ones actionable by a dependency change. **Fixed by removal, not a
version bump.**

`crates/fabric-daemon/Cargo.toml` declared four otel crates that
were never used. Verified before touching anything:
`grep -rniI 'otel|opentele'` over `src/` and `tests/` → **0
matches**, and the only 4 references in the crate were the manifest
lines themselves. `fabric-daemon` was also the sole workspace crate
declaring any opentelemetry dependency.

The version skew was self-inflicted: one manifest pinned
`opentelemetry*` at `0.27` while `opentelemetry-otlp` sat at `0.33`,
and `tracing-opentelemetry 0.28` independently held
`opentelemetry_sdk` at 0.27.1 — so bumping only the two direct pins
would never have cleared the advisory.

Result: **13 packages removed, 0 added**, including the vulnerable
`opentelemetry_sdk 0.27.1` and the redundant `0.33.1` duplicate.

**Verified after the change:**

| Gate | Result |
|---|---|
| `cargo check --workspace --all-targets` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo test -p fabric-daemon` | 8 tests + 1 doc-test pass |
| `cargo deny check advisories` / `bans` / `licenses` | all ok |
| CI workflow `38816418841` | **success** (Check, Format, Clippy, Release build, Integration, Unit, E2E smoke) |
| Dependency Audit `38816418843` | **success** (cargo-deny + Go vuln check) |
| E2E workflow `38816418828` | **success** (smoke + daemon binary) |

**Open alerts are now 2, down from 4** — `glib` #1 and #8 only,
both proven unactionable in the section below. GitHub closed the
otel alerts automatically once the vulnerable crates left the
lockfile.

**R38 analysis finding — the glib bump is purely transitive, so
nobody in this repo can force it directly.** Verified with
`cargo tree -i glib --offline --target all`:

```
glib v0.18.5
├── atk v0.18.2 → gtk v0.18.2 → { libappindicator, muda, tao }
│      └── tao / libappindicator → tray-icon v0.19.3 → fabric-tray
├── cairo-rs v0.18.5
├── gdk, gdk-pixbuf, gio v0.18.x
```

| Fact | Evidence |
|---|---|
| **No direct `glib` dependency anywhere** | `grep -rn glib --include=Cargo.toml` → **0 hits** |
| Entirely transitive via the GTK stack | `cargo tree -i glib --target all` → glib ← atk/cairo-rs/gdk/gio ← gtk ← `tao`/`tray-icon` ← **`fabric-tray`** |
| Only first-party entry point | `crates/fabric-tray` → propagates to `fabric-gui`, `fabric-web` |
| Not built in the default workspace target | plain `cargo tree -i glib` → *"nothing to print"*; needs `--target all` (Linux-only path) |
| `src-tauri` is excluded from the workspace | root `Cargo.toml` `exclude = ["crates/fabric-frame-transport/fuzz", "crates/fabric-gui/src-tauri"]`, and it declares its own empty `[workspace]` |
| `tauri` is absent from the root lockfile | `grep -c '^name = "tauri"' Cargo.lock` → **0** |

**Consequence for the decision:** glib cannot be bumped by editing
our manifests at all — it is set by `tao 0.37.1` / `tray-icon
0.19.3`, which come in via `fabric-tray`. Moving `glib 0.18.5 →
0.20.0` means bumping **`tao`** (and likely `tray-icon`) to
versions that pull the newer GTK bindings. That is why R31's
`update-types: [minor, patch]` filter blocked it: the real
constraint is upstream tao, and glib's own advisory is only
reachable through it.

**This also explains issue #32's failure pattern** (item 6). The
nested `src-tauri` directory has its own workspace and is excluded
from the root, which is exactly the layout Dependabot's
`check_required_files` cannot resolve — hence `No Cargo.toml!`
for jobs rooted there while the `/`-rooted job succeeds.

So the honest ranking for the glib alerts is:

- (a1) **Drop them** — dismiss #1/#8 as won't-fix. **Now the only
  viable option**; (a2) was attempted in R39 and proven impossible
  (see below). Cheapest correct answer.
- (a2) Bump `tao` / `tray-icon` — ~~real GTK generation bump~~
  **RULED OUT in R39: glib 0.20 requires GTK4, and tao is already
  at its final release.**
- (b) Document as accepted risk.
- (c) Wait — not applicable; 0.20.0 exists upstream, but it is
  unreachable through any available dependency version.

#### R39: (a2) ATTEMPTED AND PROVEN IMPOSSIBLE — glib 0.20 is unreachable

Operator chose the upgrade path. **It was attempted empirically
and it cannot work.** Evidence, in order:

**1. `tao` has no newer release.** `cargo info tao` → latest is
**0.37.1**, which is what we already pin. `tao = "^0.42"` fails to
resolve — no such version exists. tao is the crate holding glib at
0.18 in the root graph, and it cannot move.

**2. `tray-icon` bumps, but changes nothing that matters.**
Bumping `tray-icon 0.19 → 0.24.2` resolves cleanly and also
lifts `muda 0.15.3 → 0.19.3`, but glib stays pinned at **0.18.5**
and `gtk` at 0.18.2, because both `tao 0.37.1` and `gtk 0.18.2`
still require the 0.18 line.

**3. The decisive constraint — glib 0.20 needs GTK4, not GTK3.**

| Crate | Our version | Latest available |
|---|---|---|
| `glib` | 0.18.5 | **0.22.10** |
| `gtk` (GTK3) | 0.18.2 | **0.19.0** — the entire GTK3 line stops here |
| `gtk4` (GTK4) | n/a | 0.11.5 |
| `tao` | 0.37.1 | **0.37.1** — already latest |

The GTK3 binding line (`gtk`) tops out at 0.19.0, and no GTK3
release uses glib 0.20. glib 0.20+ exists only through **`gtk4`**,
a different, non-compatible C API. Meanwhile `tao` — our
windowing crate, and the only route to glib in the root graph — is
already at its final release and does not adopt GTK4.

**Direct proof of the version gap** (read from the vendored crate
source, not prose): `gtk-0.19.0/Cargo.toml` declares

```toml
[dependencies.glib]
version = "0.22"
```

The GTK3 line therefore jumps `^0.18` → `^0.22` with **no
intermediate release**. There is no `gtk` version that requires
glib `0.20`, so the patched version the advisory names is not
reachable through any GTK3 release at any point in time.

**Conclusion: glib 0.20 is unreachable in this project** without
replacing the entire windowing layer (tao + tray-icon + muda +
Tauri) with GTK4-based alternatives. That is not a dependency
bump; it is a platform migration. The alerts cannot be remediated
by upgrading.

**The advisory is also unreachable in our code** — a stronger
justification than "can't upgrade". RUSTSEC-2024-0429 concerns
unsoundness in **`glib::VariantStrIter`**. Verified:
`grep -rn VariantStrIter --include=*.rs .` → **0 matches**. The
API the advisory describes is never called anywhere in this
repository, so there is no reachable code path regardless of
version.

This confirms and strengthens option (a1). The correct action is
to **dismiss #1 and #8 as wont_fix** with both justifications
recorded on the alerts:

1. glib 0.20 is unreachable (GTK3 line skips it; tao is final)
2. the vulnerable `VariantStrIter` API is never called here

That is an accurate, auditable disposition rather than a silent
suppression. The `webkit2gtk 2.0.2` in the nested `src-tauri`
graph is likewise pinned to glib 0.18 and non-optional via
`tauri-runtime-wry`, so the nested sub-workspace shares the same
ceiling.

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

### 6. Issue #32 is STILL BROKEN — confirmed live, 2026-10-15T21:38Z (R38)

**Status change: issue #32 must stay open.** The previous handoff
was right that the bug is unsilenced, but framed it as a
Monday-cron question. It is not — **it is recurring on an ad-hoc
security-update trigger, and R31's `ignore` entries did not stop
it.** Observed during this session:

| Run | Job name | Conclusion |
|---|---|---|
| 38811390435 | `cargo in /crates/fabric-daemon, /crates/fabric-gui/src-tauri for glib, opentelemetry_sdk - Update #1630218070` | failure |
| 38811390474 | `... - Update #1630218069` | failure |
| 38811390741 | `... - Update #1630218068` | failure |
| 38811390419 | `cargo in /. for glib, opentelemetry_sdk - Update #1630218067` | **success** |

**7 of 7** `src-tauri` ad-hoc jobs in the last 24h failed. Root
cause is byte-identical to issue #32's original report:

```
updater | ERROR <job_1630218068> No Cargo.toml!
  .../cargo/file_parser.rb:430 Dependabot::Cargo::FileParser...
```

**The tell is in the contrast.** The sibling job rooted at `/`
(run `38811390419`, same package pair, same trigger time) passes,
while the ones rooted at `/crates/fabric-daemon,
/crates/fabric-gui/src-tauri` fail. So it is specifically Dependabot
failing to resolve manifests at those nested directories, not a
general cargo problem and not the package versions.

**What this proves about the old gate:** R31 added
`ignore: [{dependency-name: glib, security-updates-only: true}, ...]`
hoping to suppress these. They did not suppress them. Dependabot
still synthesises the ad-hoc security update, still points it at
the nested directories, still fails. Waiting until Monday would
have produced exactly the same failure and could have been
mistaken for "the fix did not work".

**Do NOT close #32.** Correct gate going forward:

```bash
# Ad-hoc security-update jobs rooted at the nested dirs must stop
# recurring. Count failures, not overall run colour.
gh run list --workflow="Dependabot Updates" \
  --created ">=2026-10-20" --limit 100 \
  | grep "src-tauri" | grep -c failure
```

- **0** → resolved; then and only then close #32.
- **>0** → still broken; the ignore approach has been disproven.

**The real fix is upstream/config, not ignores.**
`dependabot/dependabot-core` #13345, fixed in PR #13359, but the
runtime still reproduces it. The structural cause is that
`src-tauri` is a nested manifest that Dependabot's
`check_required_files` cannot resolve at the configured directory.
Candidate remedies, all needing operator buy-in:

- (a) Declare explicit `directories:` entries per ecosystem so
  Dependabot parses each manifest directly
- (b) Add `source`/`directory` mapping for the workspace layout
- (c) Make `src-tauri` a path dependency of a workspace member so
  the root manifest owns it
- (d) Accept the failures as cosmetic noise (alerts are handled
  manually anyway) and close #32 as wontfix

**No work is lost** — these jobs only ever fail before opening a
PR, and the 4 real alerts are being tracked manually. This is
hygiene, not an active security regression.

### 7. cargo-deny: REAL root cause found — a `deny.toml` schema migration (R38 correction)

**This item was written twice, and the second version is also
wrong. Adversarial review caught it. Final version below.**

**The actual root cause is NOT version drift.** Commit
`33164d7` ("fix(deps): cargo-deny v2 schema migration and 7
unmaintained advisories", 2026-10-14T17:25Z) migrated `deny.toml`
from the cargo-deny 0.20 schema to v2 — removing
`[advisories] vulnerability` and `[licenses] deny`, and adding
`version = 2` to both sections. That migration is what fixed the
`"expected a `WITH` here"` parse failure.

**Timeline, which settles it:**

| When | What |
|---|---|
| 2026-10-13T05:28Z | run `38351585765` — cargo-deny **failing** |
| 2026-10-14T17:25Z | `33164d7` migrates `deny.toml` to v2 schema |
| 2026-10-15T00:31Z | run `38660507431` — cargo-deny **green**, on unpinned install |
| 2026-10-15T14:38Z | `005e343` (R38) pins cargo-deny to 0.20.2 — **21h after** it was already green |

My earlier claim that the green streak was evidence *for* the pin
is causally false: the streak is caused by `33164d7` and predates
the pin by a day. Version drift also predicts *intermittent*
failure; the observed pattern is a clean linear green streak,
which is the signature of a fixed config defect.

**What R38's pin is actually worth:** still worth having as
hygiene — it makes the gate deterministic instead of dependent on
which cargo-deny release the runner happens to fetch. But it is
**not** the fix, and the R38 commit message overstated the case.
The pin did no harm; the attribution did.

**Verified state:** manual `workflow_dispatch` run `38811471799`
on ubuntu-latest is fully green — cargo-deny 0.20.2 installed, all
four checks pass, Go job green.

**Second error found in R38's alert accounting** (see item 4):
R38 claimed `ratatui` alerts were "already closed". **No ratatui
alert has ever existed** — `?state=all` returns no rows. ratatui
0.30.2 in the lockfile is an upgrade, not an alert resolution. The
real closed set is 5 alerts: `rustls #9`, `jsonwebtoken #7`/`#6`,
`protobuf #3`, `lru #2`. R38 omitted `rustls #9`.

**Two lockfiles, not one.** `glib 0.18.5` is pinned in **both**
`Cargo.lock` and `crates/fabric-gui/src-tauri/Cargo.lock`. Any glib
work must account for the excluded sub-workspace too.

### 8. Go vulnerability check — ALREADY FIXED on push runs; cron record is stale (R38)

The `go-audit` job failed in all 5 scheduled runs in item 7's
table, but that history is **stale**. Root cause, from run
38290237246:

```
go install golang.org/x/vuln/cmd/govulncheck@latest:
  golang.org/x/vuln@v1.9.0 requires go >= 1.26.0
  (running go 1.21.13; GOTOOLCHAIN=local)
```

The workflow ran `@latest`, which floated to a release needing a
far newer Go than the pinned `go-version: "1.21"`. Commit
`2531fb4` ("fix(ci): pin govulncheck to v1.0.4", 2026-10-14) pinned
it, and **`Dependency Audit` has been green on every push run
since** — 3 consecutive successes, most recently run `38697946861`
at 2026-10-15T07:19Z.

So this item is **not** an open defect. The only open question is
whether the *scheduled* Monday run is green, since every scheduled
run in the visible history predates the pin. That resolves on
2026-10-20. No action needed now.

Correction to my own first pass at this item: I initially
recorded the Go audit as an unexplained weekly failure alongside
the cargo-deny no-op. The two had different causes and different
statuses — the cargo-deny job is still broken by version drift
(item 7), the Go job was already fixed a day ago.

### 9. 3 stale remote branches (R35)

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
gh api /repos/KooshaPari/PhenoFabric/dependabot/alerts \
  | jq '[.[] | select(.state == "open")] | length'
# Expected: 2  (glib #1, #8 — see R40/R39 for why they cannot be upgraded)

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

1. **Confirm the R38 changes on CI** — the `deny.yml` pin and the
   `deny.toml` cleanup were verified locally on macOS; watch the
   next `Dependency Audit` run to see 0.20.2 install and all four
   checks pass on Linux.
2. **Verify the Mon 2026-10-20 dependabot run** against the
   corrected gate in item 6 — check for a recurring ad-hoc
   `glib`/`opentelemetry_sdk` job, not just an overall green run.
3. **Decide the `glib` / `opentelemetry_sdk` bumps** (item 4).
   Both have patched versions; this is ordinary dependency work.
4. **Re-check `RUSTSEC-2023-0071`** at the next lockfile refresh —
   annotated in `deny.toml` as no-longer-matching.
5. **Document v0.1.2.x release plan** once the docker fix
   forward-port is decided.
6. **Prepare v0.1.3** if the operator wants to move past the
   partial v0.1.2.

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
