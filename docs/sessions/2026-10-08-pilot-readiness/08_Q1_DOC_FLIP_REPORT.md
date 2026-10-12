# Q1.A Doc-Flip Report — 2026-10-11 23:45 PDT

## Commit

- **SHA:** 464dc69
- **push:** ok (remote: `ce8dda1..464dc69 research/transport-baseline-2026-10-08`)
- **subject:** docs(pilot-readiness): slice #1 — flip to BEST-EFFORT (characterization harness)

## README changes (`docs/research/2026-10-08-transport-baseline/README.md`)

- Status line flipped from "GATES PASS, NO MUTATION PROOF" to
  "GATES PASS, NO MUTATION PROOF — BEST-EFFORT (characterization
  harness, no assertion surface)"
- Added paragraph explaining characterization-vs-correctness gap:
  test measures RTT but does not assert on values; mutations to
  constants cannot cause test failures
- Mutation checklist entry changed from "BLOCKED: 6 consecutive
  endpoint failures" to "N/A — characterization harness, no
  assertion surface to break"
- Mutation-proof gap section references future strategies:
  (a) add assertions, (b) syscall delay injection

## Continuation brief changes (`docs/sessions/2026-10-08-pilot-readiness/04_CONTINUATION_BRIEF.md`)

- Added "Test type note (added 2026-10-11 23:41)" section between
  the state-inheritance block and the step-by-step instructions
- Explains that the test is a characterization harness, not a
  correctness test; the original Step 4 mutation recipe assumed
  assertions existed
- Documents two future mutation strategies: add assertions, or
  inject syscall-level delay
- Retains the original Step 4 recipe for reference with a note
  that it won't produce detectable failures

## Files NOT modified (read-only constraint honored)

- `crates/fabric-integration-tests/tests/transport_baseline.rs` — untouched
- All other source code — untouched

## What this closes

Q1.A of the operator MCQ: "Flip methodology README to BEST-EFFORT
(characterization harness)." Slice #1 is now honestly documented as
gates-pass / best-effort / no-assertion-surface. The operator needs
to decide Q1.B-Q1.D (whether to add assertions, whether to cite the
numbers in a pilot claim, whether to proceed with other slices).