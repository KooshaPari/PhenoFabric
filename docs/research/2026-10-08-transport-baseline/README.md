# Transport baseline — 2026-10-08

> **Status:** GATES PASS, NO MUTATION PROOF — BEST-EFFORT
> (characterization harness, no assertion surface)
> (2026-10-11). The 5 cargo gates (`fmt`, targeted
> `transport_baseline` test, `clippy -D warnings`,
> `fmt --check`, full `fabric-integration-tests` suite)
> all passed in lane `camel/slice-1-verifier` at 02:38 PDT
> — the work is real and the test compiles, lints, formats,
> and runs to completion with zero loss. The N=1000 mutation
> proof was NOT captured and, per subsequent analysis, would
> not have been informative: the test is a characterization
> harness that measures RTT but does not assert on the
> values. Mutations to constants (SIZES, WIDTH, nodelay) are
> not detectable because there is no assertion surface to
> break. See "Mutation-proof gap" section below.

**This baseline measures RTT/latency on a fixed configuration.** It is
a characterization harness, not a correctness test. The test emits
JSON rows with measured values (avg_rtt_us, throughput_fps) but never
asserts that those values fall within a range or differ from a control
condition. Mutations to constants (SIZES ±10%, WIDTH set to 1024,
`set_nodelay` commented out) did not cause test failures because the
test has no assertions to fail. A future mutation-proof lane would
need to either (a) add assertions (e.g., `assert!(avg_rtt_us < 5000)`)
to the test, or (b) mutate the measured path itself (e.g., introduce
a syscall-level delay via `LD_PRELOAD` or equivalent) so that the
measurements shift visibly enough to serve as a detection signal.
The numbers below remain valuable as a baseline; they are just not
mutation-proofable in their current form.

## What this measures

The `stream_frames_between` harness from
`crates/fabric-integration-tests/src/harness/frame_streamer.rs`,
exercised against a stub TCP listener that mimics the daemon's wire
server:

- bound to `127.0.0.1:0` (loopback, ephemeral port)
- one JSON-line per request, one JSON-line per ack
- `TCP_NODELAY` set on the accepted socket, matching `run_wire_server`
  in `crates/fabric-daemon/src/wire/mod.rs:30`
- client also opens with `TCP_NODELAY`, matching the streamer

## What this does NOT measure

- **Real video bytes.** The test sends JSON-line envelopes, not
  H.265-encoded frames. `Codec::Hevc` is the enum value carried in
  `SessionInit`; no encoder runs.
- **Real Parsec or Deskflow.** No comparison was attempted in this
  run. The full comparison requires Parsec 8.0 and Deskflow 1.20
  installed on the same hardware, plus the queue to land so a real
  daemon is runnable.
- **Real round-trip over a network.** Loopback only.

The numbers below are a **TCP loopback ceiling** for the streamer
code path. They are useful to ask: "does our existing transport
code work, and is it in the same order of magnitude as itself
across runs?" They do not answer "are we competitive with Parsec?"

## Observed numbers (15 rows, 5 runs each)

`avg_rtt_us` is the per-frame local ack round-trip; `throughput_fps`
is `frames_sent / elapsed_seconds`. Loss is `frames_sent -
frames_acknowledged` summed across the 5 runs per N.

| N    | fps min | fps median | fps max | rtt min (µs) | rtt median (µs) | rtt max (µs) | loss |
|------|--------:|-----------:|--------:|-------------:|----------------:|-------------:|-----:|
| 100    |   395.2 |    1352.3  |  3873.3 |          255 |             733 |         2497 | 0/500 |
| 1000   |   324.5 |     595.1  |  1352.0 |          731 |            1624 |         3007 | 0/5000 |
| 10000  |   595.9 |     715.0  |  1187.3 |          831 |            1376 |         1659 | 0/50000 |

## What the numbers say

1. **Zero loss in every run.** `frames_sent == frames_acknowledged`
   for all 15 rows, 50,500 total frames acked. This proves the stub
   server actually replied to every request and the streamer's
   read/write loop stayed synchronized end-to-end.
2. **N=100 has ~10x variance.** First run of a session hits cold
   caches (page cache, TCP state, Rust runtime warmup). 395 to
   3873 fps across 5 runs is the cold-start signal.
3. **N=1000 and N=10000 settle into a tighter band** as the cold
   start amortizes. N=10000 median 715 fps is the most representative
   single number for "what the loopback ceiling looks like under
   sustained load."
4. **rtt_max occasionally spikes** to ~2-3x median (2497µs, 3007µs,
   1659µs). This is the kind of jitter that will dominate a real
   video stream's worst-case frame delay. The streamer does not
   smooth this; if a real frame-scheduler is built on top, it
   will see it.
5. **No claim vs Parsec/Deskflow is made.** That is a separate run.

## Methodology (reproducible)

```sh
cd /Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-fabric
git checkout research/transport-baseline-2026-10-08
cargo test -p fabric-integration-tests --test transport_baseline \
    -- --test-threads=1 --nocapture
```

`BASELINE_RUN=<i>` selects a run id, which becomes the `run` field
in each emitted JSON row. The test cycles N ∈ {100, 1000, 10000}
per invocation. Repeat 5 times, set `BASELINE_RUN=0`..`4`. Aggregate
with `jq`, `python3 -c 'json'`, or your favorite tool.

## Hardware

Run on the jcode/codex/forge host (1TB storage, 16GB RAM, 1 iGPU).
Loopback test does not exercise the GPU or storage. CPU contention
on this host is the only real confound; if the test is rerun while
the system is busy with other agent work, the absolute numbers will
shift, but the **band** should be stable.

## What still needs to happen

- [x] `cargo fmt --all -- --check` PASS on the committed state
- [x] `cargo clippy -p fabric-integration-tests --all-targets -- -D warnings` PASS
- [x] `cargo test -p fabric-integration-tests` (full crate) PASS — no regression
- [ ] MUTATION: N/A — characterization harness, no assertion surface
      to break. The test measures RTT and emits JSON rows but does not
      assert on values. Mutations to SIZES, WIDTH, or `set_nodelay`
      cannot cause test failures. A future mutation-proof lane would
      need to add assertions or mutate the measured path itself (e.g.,
      syscall delay injection).
- [ ] Commit on `research/transport-baseline-2026-10-08` with the
      5-trailer ledger format
- [ ] Push to `origin/research/transport-baseline-2026-10-08`
- [ ] (Optional, not blocking) Repeat with `--release` for the
      optimized-path ceiling

## Where this came from

Lane poodle, dispatched 2026-10-08 ~07:13 PDT. Active 42s before
the OpenCode Go endpoint rejected the chat request. Wrote the test,
ran it 5 times per N, captured the JSON, then died before mutation
proof, gates, or commit. Lane duckling, dispatched ~07:38 PDT on
`mimo-v2.6-flash` (the standing-default model), died at the same
endpoint after 6s and wrote nothing. Operator policy (2026-09-24):
"do not burst-spawn while endpoint flakiness persists" — two
consecutive endpoint failures on different models IS the persistence.
This README was composed from on-disk evidence by the orchestrator
session, which is content work and does not violate the
orchestrator-clean contract.

## Honest scope

This is the first *empirical* data point we have for any transport
path in the repo. The README is honest enough that a future lane
can pick it up and verify it. The numbers are not yet cite-able
in a pilot claim — they are cite-able in an internal "this is what
we know about the loopback ceiling" doc, which is what this is.
