# Inventory + Decision Report — 2026-10-11 23:22 PT

Subagent: read-only inventory worker (dispatched from orchestrator session).
Generated: 2026-10-11T23:22:26Z, written to disk at 2026-10-11T23:36Z.

---

## Ground Truth (verified + extended)

| Field | Value |
|---|---|
| Repo | `phenotype-fabric` (NOT `repos/` root) |
| Branch | `research/transport-baseline-2026-10-08` |
| HEAD | `203b551` — Revert "merge pr-22 onto a27f815 (rebase-style, no force-push)" |
| Working tree | 2 untracked files: `05_OPERATOR_MCQ.md`, `06_OPERATOR_DECISIONS_RECORD.md` |
| Local commits since `ce8dda1` | `d557af9` merge pr-22 onto a27f815 (rebase-style, no force-push), `203b551` Revert "merge pr-22 onto a27f815 (rebase-style, no force-push)" |
| Remote rebase/pr-21-onto-a27f815 | `c539d5e340a0462d135d65c3aca502da385f85dc` |
| Remote rebase/pr-22-onto-a27f815 | `e855f7c57bd0f657227c089faac8146d1d88bf81` |
| transport_baseline.rs MUTATION count | 0 (`grep -c MUTATED` returns exit 1 = no match) |
| Disk | 115Gi free of 926Gi |
| Net diff ce8dda1..HEAD | **zero** (merge + revert cancel out) |

### 06_OPERATOR_DECISIONS_RECORD.md (full content)

```
Q1.A [dispatch on next fallback] + Q1.a.2 [opencode-go:mimo-v2.5-pro] | Q2.A [unhook PR #21/#22 merge gate, dispatch rebase lane] | Q3.A [unhook Renovate gate, squash-merge #11-#18] | Merge-gate probe: INCONCLUSIVE (no hook config on disk; prior attempt blocked by pre_tool hook request_id=hook-538d2362a5467290976e0c05924f7340; 8 PRs MERGEABLE, hook-gated, merge_attempts: 0)
```

### 01_LANE_STATUS.md — kikazaru/iwazaru/v2.5 search

**No matches found.** `grep -ri "kikazaru\|iwazaru\|v2\.5"` across the entire session directory returned zero results. The subagents did not write self-reports to this file.

---

## Rebase Branch Contents

### rebase/pr-21-onto-a27f815 (c539d5e)

Merge commit onto a27f815. Adds fail-closed base URL seam test + rustfmt fix.

| File | Change |
|---|---|
| `crates/fabric-daemon/src/main.rs` | -30/+35 |
| `crates/fabric-daemon/src/secret_loader/env_wiring.rs` | +79 (new) |
| `crates/fabric-daemon/src/secret_loader/mod.rs` | +48 (new) |
| **Total** | 3 files, +132/-30 |

Content: Extracts `env_wiring` module, adds two seam tests (`startup_seam_rejected_base_url_attempts_no_fetch`, `startup_seam_valid_base_url_attempts_fetch_at_configured_origin`), applies rustfmt to satisfy CI Format gate.

### rebase/pr-22-onto-a27f815 (e855f7c)

Merge commit onto a27f815. Adds `pub(super) reject_token_on_forbidden` helper + delegates 4 v1 read sites.

| File | Change |
|---|---|
| `crates/fabric-daemon/src/auth/secret_read.rs` | -18/+5 approx |
| `crates/fabric-daemon/src/auth/secrets.rs` | +72/-42 approx |
| **Total** | 2 files, +48/-42 |

Content: Consolidates 401/403 token eviction into shared `reject_token_on_forbidden` helper in `secrets.rs`. 4 doc comments reference `contract_hardening.rs:134`. Status-only auth errors (no response body propagation). Protected files (contract_hardening, review_response, mod.rs, tests, docs) kept at main's version.

---

## Existing PRs from Rebase Branches

**No PRs exist from `rebase/*` branches.** The `gh pr list` output shows zero entries with `headRefName` starting with `rebase/`.

Original PRs still OPEN:

| PR | headRefName | State |
|---|---|---|
| #21 | `test/cmd-start-base-url-fail-closed` | OPEN (CONFLICTING on original base) |
| #22 | `fix/legacy-v1-token-eviction` | OPEN (CONFLICTING on original base) |

Renovate PRs #8-#18: all OPEN, all hook-gated, `merge_attempts: 0`.

---

## Decision Recommendations

### Q1 (slice #1 mutation proof): MARK-BEST-EFFORT-FAILED

**Rationale:**
- The characterization harness (`transport_baseline.rs`) has **no assertion-bearing mutation surface**. The recipe assumed assertions existed; they do not (it's a characterization harness that prints/records, not asserts).
- Seven consecutive OpenCode Go endpoint failures exhausted the provider budget for this session.
- The continuation brief in `04_CONTINUATION_BRIEF.md` is provider-agnostic but **recipe-flawed** — it assumes the test has assertions to flip.
- A future session would need to: (a) add an assertion to the harness first, then run mutation proof on a working provider, OR (b) accept characterization-only evidence as the floor.

**Action:** Flip methodology README to "BEST-EFFORT: 5/5 gates pass, 0/1 mutation proof (characterization harness limitation + 7 provider flakes)" and update lane-status doc.

### Q2 (PR rebase): OPEN-NEW-PRS-FROM-REBASE-BRANCHES

**Rationale:**
- The rebase branches are clean, contain the correct work, and are already pushed to remote.
- The original #21/#22 are CONFLICTING on their old bases; the rebase branches resolve this.
- `gh pr create` will be attempted. The pre_tool hook (`request_id=hook-538d2362a5467290976e0c05924f7340`) will likely defer again.
- If the hook fires, report the deferral + hook request_id so the operator can decide whether it's an intentional gate or stale lock.

**Action:** Dispatch "pr-rebase-creator" subagent to run `gh pr create` for both rebase branches against `main`. Expect hook deferral; report it.

### Q3 (Renovate batch): DISPATCH-BATCH-DESPITE-HOOK-EXPECTING-DEFER

**Rationale:**
- Operator ticked Q3.A: unhook Renovate gate, squash-merge #11-#18.
- Same hook issue as Q2. The operator needs to see a fresh attempt fire (or fail) to decide whether to physically remove the hook.
- 8 PRs (#11-#18) are all MERGEABLE, all dependabot cargo bumps.

**Action:** Dispatch "renovate-merger" subagent to attempt squash-merging #11-#18. If hook fires, report deferral.

### Local cleanup (d557af9/203b551): LEAVE-AS-IS

**Rationale:**
- The 2 commits net to zero diff vs `ce8dda1` (merge + revert cancel out).
- `git reset --hard` is policy-blocked. Double-revert adds noise. Not worth it.
- The commits are local-only (never pushed to this branch's remote tracking).
- The real work lives on the remote rebase branches, not this local chain.
- The 2 untracked doc files survive any reset anyway.

**Action:** None. Document the noise and move on.

---

## Recommended Dispatch Chain (priority order)

1. **pr-rebase-creator** — Attempt `gh pr create` for `rebase/pr-21-onto-a27f815` and `rebase/pr-22-onto-a27f815` against `main`. Close original #21/#22 if new PRs succeed. Expect hook deferral.
2. **renovate-merger** — Attempt squash-merge of PRs #11-#18. Expect hook deferral.
3. **docs-flip** — Update methodology README to "BEST-EFFORT" status. Update `01_LANE_STATUS.md` with final verdict. Commit and push on the research branch.
4. **Local cleanup** — Skip. Leave d557af9/203b551 as-is.

---

## Bottom Line

The rebase branches carry the real Q2 work and are ready for PR creation; the hook will likely gate `gh pr create` and the operator needs to see that happen on a fresh attempt to decide. Q3 is the same hook story with Renovate. Q1 is a methodology mismatch (characterization harness vs assertion-based mutation recipe) compounded by 7 provider flakes; mark it best-effort and move on. The local branch has 2 zero-diff noise commits that aren't worth cleaning up. Priority dispatch: pr-rebase-creator first (Q2), renovate-merger second (Q3), docs-flip third (Q1).