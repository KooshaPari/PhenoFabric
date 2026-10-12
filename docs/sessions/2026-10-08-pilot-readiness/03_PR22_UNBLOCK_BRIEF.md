# Post-#20 analysis — PR #22 unblock plan (lane-ready)

> **Purpose:** capture the analysis I would have done in a lane, so
> the next lane that runs the P2 brief can pick this up without
> re-deriving the reasoning. Parent-clean (content only, no source
> edits, no gates).

## What the operator asked

"Take the best long-term path." I evaluated four options. The
combined path that costs the least and closes the most security
debt is: (1) decide the 403 question, (2) fix the third-copy of
the eviction helper, (3) rebase PRs #21 and #22 onto a27f815,
(4) address CHANGES_REQUESTED comments, all in one lane.

## 403 design call — RESOLVED from existing code

The merged main at a27f815 contains
`crates/fabric-daemon/src/auth/secret_read.rs:52 (comment) and :63 (check)`
which documents the design explicitly:

> "403 is the same failure mode (token valid, secret not permitted)
> and must not be reported as an opaque operation failure."

This means the repo's own design is: **401 and 403 both indicate
the token cannot reach the requested secret. Both evict.** This is
the design PR #22 follows.

CodeRabbit's finding on `secret_read.rs:64-66` ("403 evicts a token
that is probably still valid") is therefore a misread of the
existing design — it runs against the line the repo just
documented at secret_read.rs:52 (comment) and :63 (check).

**Decision: do not change PR #22's 401+403 eviction. Add a one-line
doc comment at the v1 read sites in secrets.rs (~lines 280, 320,
380, 407) referencing secret_read.rs:52 (comment) and :63 (check) so a future
reviewer reading the v1 reads in isolation has the same context
the v4 tests now have.** This is the minimum-cost resolution.

The mechanical refactor (third copy of `reject_token_on_forbidden`)
is then the actual change:

- `reject_token_on_forbidden` is currently `fn` in `secrets.rs`
  (no visibility prefix → private to that module).
- The v4 read in `secret_read.rs:64-66` re-implements the same
  logic inline.
- Move: make it `pub(super)` in secrets.rs, call it from
  `secret_read.rs`, delete the inline copy. The `token` field
  can go back to private visibility.

Both changes fit in one commit. One mutation proof is required:
force the helper to return early without evicting, observe
`v1_read_401_does_not_replay` and `v4_read_401_does_not_retry`
both fail. That covers both call sites with one mutation.

## Rebase plan (mechanical)

Both PRs are DIRTY against a27f815. The conflict is in:

- `crates/fabric-daemon/src/auth/secret_read.rs` — the v4 read
  path. PR #20 added `reject_token_on_forbidden` to secrets.rs
  and made the v4 read in secret_read.rs inline the same logic
  (this is what CodeRabbit flagged as a third copy).
- `crates/fabric-daemon/src/secret_loader/contract_hardening.rs`
  — the new test file from PR #20. PR #21 has overlapping
  contract tests that may or may not duplicate.
- `crates/fabric-daemon/src/secret_loader/env_wiring.rs` — the
  real cmd_start seam test. PR #20 may have reshaped this.

Rebase approach for both:
1. `git fetch origin main`
2. `git checkout <pr-branch> && git rebase origin/main`
3. Resolve by taking PR #20's version of the seam/contract
   test files (they are the protected branch's truth), and
   keeping each PR's substantive change.
4. `cargo fmt`, `cargo clippy -D warnings`,
   `cargo test -p fabric-daemon` must pass on the rebased
   branch.

## CHANGES_REQUESTED resolutions after rebase

For PR #22 (after rebase + the 401+403 doc + the
pub(super) refactor):
- CodeRabbit #1 (third-copy helper) → addressed by the
  pub(super) refactor.
- CodeRabbit #2 (403 evicts valid token) → addressed by
  the new doc comment at each v1 read site.
- GitGuardian → re-runs cleanly because the rename
  commit 64dd4bb is already in the history and PR #20's
  merge commit is also in the new base.

For PR #21 (after rebase):
- CodeRabbit #1 (preconfigured-secret preservation) →
  test only; the fix in PR #20 already covers this. The
  test should be re-run to confirm it still passes against
  the post-#20 base.
- CodeRabbit #2 (env=dev warning) → may already be
  addressed by PR #20's review_response.rs test coverage;
  if not, file a follow-up commit in this lane.

## Lane prompt (one turn)

You are a worker on a fresh provider (Mimo 2.6 via the
operator-authorized provider, NOT opencode-go). Working dir is
the research/transport-baseline-2026-10-08 branch. Do NOT
checkout the slice #1 work — you have a different scope today.

Goal: close the 403 question, do the pub(super) refactor, rebase
PRs #21 and #22 onto a27f815, push, report SHAs and gate
results.

Steps (≤10 minutes each):
1. `git fetch origin`
2. Read the four files listed in "Rebase plan" above and the
   CHANGES_REQUESTED review comments for PRs #21 and #22 (use
   `gh api repos/KooshaPari/PhenoFabric/pulls/22/comments`).
3. Create a feature branch from origin/main: `git checkout -b
   fix/pr22-review-comments origin/main`. Do NOT work on the
   PRs' branches directly.
4. Implement the pub(super) refactor: promote
   `reject_token_on_forbidden` in `secrets.rs` to `pub(super)`,
   import it into `secret_read.rs`, delete the inline copy.
   Add the four one-line doc comments at the v1 read sites
   referencing `secret_read.rs:52 (comment) and :63 (check)`.
5. Mutation proof: in `secrets.rs`, change the helper to
   `return Ok(())` instead of clearing the token. Run
   `cargo test -p fabric-daemon --lib v1_read_401 v4_read_401`.
   Both should FAIL. Revert.
6. `cargo fmt --all -- --check`, `cargo clippy -p fabric-daemon
   --all-targets -- -D warnings`, `cargo test -p fabric-daemon`.
   All must pass.
7. Commit with the five tx-* trailers and author
   `KooshaPari <koosha@example.com>`.
8. Push the branch. Do NOT open a PR — that requires the
   operator hook.
9. Separately: `git fetch origin pull/21/head:pr21 && git
   fetch origin pull/22/head:pr22`. For each, in a fresh
   worktree, run `git rebase origin/main`, resolve conflicts
   per the plan above, then push to a NEW branch name
   (`pr21-on-a27f815` and `pr22-on-a27f815`). The destructive-
   command blocklist forbids `--force`/`--force-with-lease`
   even on PRs; the new branch name avoids that need
   entirely. The original PR branches (`fix/legacy-v1-token-
   eviction`, `test/cmd-start-base-url-fail-closed`) stay
   untouched so the operator can close them as superseded
   and use the new branches in their place.
10. Report: SHAs of (a) the fix commit on the feature branch,
    (b) the rebased PR #21, (c) the rebased PR #22. Plus a
    summary of any conflict resolutions that were not the
    "take main's version" default.

## What I am NOT doing in the parent

- I am not running cargo test, clippy, or fmt in the parent.
- I am not editing any source file in the parent. (I
  considered writing the diffs here, but that would be
  orchestrator-dirty — the diffs need to be tested on a
  worker.)
- I am not opening PRs. The hooks (hook-dfc5b037,
  hook-291e66a) require operator approval.

## What the operator needs to do (one decision)

Pick a provider for the lane dispatch. The MiniMax via
`https://api.minimax.io/v1` is the named alternate. Or wait
for OpenCode Go to recover. Either way, the brief above is
one-turn runnable and will close two security PRs + the
research slice.
