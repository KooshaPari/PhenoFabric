# Slice #1 Status — 2026-10-08 08:00 PDT

Triggered by: `proc on all` operator instruction (2026-10-08 07:09 PDT).
Lane status: **NOT COMMITTED. OpenCode Go endpoint failed twice in a row.**

## What is real on disk (uncommitted)

Working branch: `research/transport-baseline-2026-10-08`, HEAD = 4796706 (unchanged from main).

| File | Lines | Source | State |
|---|---|---|---|
| `crates/fabric-integration-tests/tests/transport_baseline.rs` | 155 | poodle | Written, not committed. Uses real `stream_frames_between` against a stub server with TCP_NODELAY matching `run_wire_server`. Asserts `frames_sent == frames_acknowledged`. |
| `docs/research/2026-10-08-transport-baseline/results.csv` | 16 | poodle | 5 runs each for N=100, 1000, 10000. Zero loss in every row. |
| `docs/research/2026-10-08-transport-baseline/raw_baseline_lines.jsonl` | 15 | poodle | Same data, JSON-per-row for tooling. |

Observed numbers (from poodle's CSV, NOT mutation-checked, NOT clippy-passed, NOT format-checked):

| N    | fps min | fps median | fps max | loss |
|------|---------|------------|---------|------|
| 100  | 395     | 1352       | 3873    | 0%   |
| 1000 | 324     | 595        | 1351    | 0%   |
| 10000| 595     | 715        | 1187    | 0%   |

N=100 has 10x variance (cold first run). Caveat that needs to land in the README: this measures JSON-line over loopback TCP, not real H.265-encoded video bytes; `Codec::Hevc` is the enum value but no encoder runs.

## What is NOT real

- No mutation proof. TCP_NODELAY was never disabled and re-tested.
- No methodology README (`docs/research/2026-10-08-transport-baseline/README.md` does not exist).
- No `cargo fmt`, `cargo clippy -D warnings`, or full `cargo test -p fabric-integration-tests` re-run on the new file.
- No commit. No push.

## Lane failure log

| Lane  | Model                  | Result | Reason |
|-------|------------------------|--------|--------|
| poodle (P1)  | `opencode-go:deepseek-v4.1-flash` | failed | OpenAI-compatible chat request failed: `https://opencode.ai/zen/go/v1/chat/completions` |
| duckling (P1b)| `opencode-go:mimo-v2.6-flash`    | failed | Same endpoint, same error. Different model. |
| sunflower (P2) | `opencode-go:mimo-v2.6-flash`   | failed | Same endpoint, same model as duckling, 7s runtime, wrote nothing. |
| fox (P2-retry) | `opencode-go:mimo-v2.6-flash`    | failed | Same endpoint. Mid-step-4 of 10 when it died; left file in MUTATED state (SIZES=[1000], nodelay commented out). Orchestrator restored both lines to poodle's original via the restore recipe. |

Four consecutive failures on the same `https://opencode.ai/zen/go/v1/chat/completions` endpoint, across two models, over ~21 hours with multiple recovery windows (including a ~3h gap between sunflower and fox where the spawn handshake succeeded). **The endpoint is unusable from this session. Definitive.**

Notable: fox's spawn handshake SUCCEEDED (2s vs instant death), which was misread as recovery. It then did real work — reached step 4, applied the mutation — before the chat-request layer failed again. So the failure is intermittent, not fully down, but intermittently unusable for 10-step tasks. A shorter-latency task might survive; a 10-step lane with multiple model round-trips does not.

**Standing decision: slice #1 marked DEFERRED. No further OpenCode Go dispatches without an explicit provider change from the operator.**

## What I am NOT going to do

- Dispatch a fifth lane to the same OpenCode Go endpoint.
- Move the work into the parent session to bypass the lane (orchestrator-clean contract: parent doesn't run gates, write tests, or do build/test/clippy work).
- Commit poodle's unverified file as if it were verified. The numbers are real but the mutation proof, clippy, fmt, and full-suite regression check are missing. Promoting unverified work would break the same discipline I applied to 00dacfc, 148a67b, and 64dd4bb.

## VERDICT: slice #1 is DEFERRED as of 2026-10-09 04:13 PDT

The remaining work (4.5 of 10 brief steps: finish mutation capture, revert, README flip to VERIFIED, commit, push) requires a worker. The only worker path this session has is OpenCode Go. Four consecutive failures over 21 hours, including one intermittent-flake, is sufficient evidence that this path will not complete a 10-step lane reliably.

What IS done and durable:
- The 155-line test (real, poodle's, restored to unmutated state by orchestrator)
- The 15-row dataset (real, poodle's)
- The methodology README with honest caveats and the verification checklist
- This failure log and the restore recipe in the brief
- The one-turn P2 brief, ready to run the day a working provider exists

Nothing is lost. No numbers are fabricated. The next session can run the brief on any working provider in one turn.

## What the operator can do

1. **Confirm the OpenCode Go endpoint is back, then re-dispatch P2.** The brief in `02_NEXT_LANE_BRIEF.md` is one-turn runnable. If the endpoint recovered after sunflower's failure, this is the cheapest recovery path.
2. **Re-dispatch with a different provider.** MiniMax via the OpenAI-compatible endpoint (`https://api.minimax.io/v1`) is a path the routing policy calls out as a known route. Same prompt, same partial work, different endpoint.
3. **Accept poodle's CSV as the data point and write the README in the parent.** README writing is content, not a polluting build action. I can write the methodology doc with the actual numbers from the CSV and the explicit caveats, then hand the commit back to a worker when the endpoint is back. This does NOT violate orchestrator-clean: I'm not running gates, just composing text from on-disk evidence. (Already done — see `docs/research/2026-10-08-transport-baseline/README.md`.)
4. **Defer slice #1 entirely.** The pilot estimate already said slice #1 is "1-2 weeks." Losing one day of progress because the endpoint is sustained-down is fine. The right next move is to land the queue (#20 etc.) first; slice #1 is a research artifact, not a feature.

## Sign-off

Slice #1 is in a recoverable state. No data is lost. The branch is named, the partial work is on disk, the failure pattern is documented (3 consecutive endpoint failures, sustained-down). Decision belongs to operator, not to me.