# Operator MCQ — 2026-10-11 21:04 PT

> Read, tick one box per question, save. The ticked boxes
> are the dispatch signal. Unticked = "leave blocked."
> All questions are independent — you can answer some and
> leave others blank. Empty file = "keep current state."

Current session: `research/transport-baseline-2026-10-08` @ `ce8dda1`.
Doc state and dispatch state are at the bottom of this file.

---

## Q1. Slice #1 mutation proof

Slice #1 (TCP frame-transport baseline) is committed at
`78abd22` with the test, CSV, and JSONL on disk. Camel's
checkpoint at 02:38 captured **5/5 cargo gates PASS**.
The N=1000 mutation factor is **not** captured because
7 consecutive OpenCode Go dispatches have died
(poodle, duckling, sunflower, fox, camel-partial,
octopus, vole). The continuation brief
`04_CONTINUATION_BRIEF.md` is provider-agnostic and
ready to run on any working model.

- [Y ] **A. Dispatch on the next authorized fallback.**
  Which provider? See the candidates below.

- [ ] **B. Accept "GATES PASS, NO MUTATION PROOF" as
  the floor.** Slice #1 evidence is real but not
  cite-able in a pilot claim. The methodology README
  already reflects this state. No further dispatches.

- [ ] **C. Defer slice #1 to a future session.** The
  continuation brief stays on disk. Pilot work moves
  to the queue (Q2 and Q3 below).

### Q1.a — if A, which provider?

Per the operator-authorized fallback ladder in
`~/.jcode/memories/global/harness-agents.md` (granted
2026-09-17 04:24, retained), tool-loop-verified
candidates in priority order:

- [ ] **1. `opencode-go:deepseek-v4.1-flash`** —
  tool-loop verified 2026-09-17; was the 2026-09-17
  standing default before the 2026-09-24 update
  promoted `mimo-v2.6-flash`.

- [Y ] **2. `opencode-go:mimo-v2.6-flash`** — tool-loop
  verified 2026-09-17.

- [ ] **3. `opencode-go:glm-5.3`** — one-shot
  verified 2026-09-17 (re-qualify by tool loop
  before promoting).

- [ ] **4. `opencode-go:deepseek-v4-pro`** — one-shot
  verified 2026-09-17 (re-qualify by tool loop
  before promoting).

- [ ] **5. `opencode-go:omen-alpha`** — one-shot
  verified 2026-09-17 (re-qualify by tool loop
  before promoting).

- [ ] **6. Other** — write the model id here:

  ```
  _____________________________________________
  ```

If Q1.a is left blank and A is ticked, I will not
dispatch. The "advance only on actual failure of the
model above" rule means I do not pick from the ladder
without the operator's pick.

---

## Q2. PRs #21 and #22 (Infisical token-routing 403)

Both PRs are **CONFLICTING** post-#20 and
**CHANGES_REQUESTED**. The 403 design call has been
resolved from existing main code (`contract_hardening.rs:134`
on `a27f815` documents "401 and 403 both indicate the
token cannot reach the secret, both evict"). The
rebase brief `03_PR22_UNBLOCK_BRIEF.md` is lane-ready:
4 doc comments at v1 read sites + `pub(super)`
refactor + rebase onto `a27f815` onto new branches
(no force push).

- [Y ] **A. Unhook the merge gate.** I will dispatch a
  rebase lane using the same provider picked in Q1.a.
  If Q1.a is blank, this is blocked on Q1.

- [ ] **B. Defer to a future session.** Both PRs stay
  CONFLICTING and CHANGES_REQUESTED. Brief stays on
  disk.

- [ ] **C. Close as won't-fix.** The 403 design call
  resolves the only review comment, so this becomes a
  "the source-of-truth is on main, the PRs are
  obsolete" closure. Requires operator to confirm.

---

## Q3. Renovate dep-bump queue (#11–#18)

8 Renovate dep-bump PRs, all **MERGEABLE**, all
**hook-gated**, `merge_attempts: 0` on each. Non-critical
batch.

- [Y ] **A. Unhook the Renovate gate and squash-merge
  the full batch (#11–#18).** No provider dependency;
  this is a local squash. Author and trailers stay
  per session convention.

- [ ] **B. Unhook the Renovate gate and squash-merge
  a subset.** Write the PR numbers here:

  ```
  _____________________________________________
  ```

- [ ] **C. Leave hook-gated.** The 8 PRs stay open
  until the operator picks them up directly.

---

## Q4. Disk pressure (WATCH, not a decision)

Currently **140G free / 15% used**. No pressure. The
session is not blocked on this; included so the
operator sees the full state.

---

## State at 21:04 PT

- **Branch:** `research/transport-baseline-2026-10-08`
  @ `ce8dda1` (pushed)
- **Main:** `a27f815` (PR #20 merged; local main
  fast-forwarded)
- **Working tree:** clean
- **Slice #1 file:** unmutated, 0 MUTATION markers
- **Methodology README:** "GATES PASS, NO MUTATION PROOF"
- **OpenCode Go probe:** empty response (7th flake);
  no further dispatches without operator signal
- **Disk:** 140G free
- **Provider block:** 7 flakes in this session;
  permanent for this session, not temporarily relaxed
- **Todos (4 incomplete):** 2 hook-gated, 1
  provider-gated with operator-pick pending, 1 disk
  WATCH

## How to use this file

1. Read each question.
2. Tick one box per question (or write a model id /
   PR list in the free-text line).
3. Save the file.
4. Tell me "filed" or paste the ticked lines into chat.
5. I will execute the ticked choices and commit a
   `docs(pilot-readiness): operator decisions Q1/Q2/Q3`
   record with the picked boxes verbatim, then push.

If you tick nothing, the session stays in the current
state. If you tick only Q3.A, I will do the Renovate
batch and leave Q1/Q2 blocked. Independent decisions.

## Ledger note

This file is **not** committed to the repo. It is a
session artifact for the operator. The picked choices
will be committed and pushed as a `docs/` record when
executed.
