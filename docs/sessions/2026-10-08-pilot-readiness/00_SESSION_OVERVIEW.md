# Pilot-Readiness Survey — 2026-10-08

Triggered by: `proc on all` operator instruction (2026-10-08 07:09 PDT).
Purpose: ground-truth what the repo can ship to a pilot *today* vs. what
needs to be built, with honest time estimates against Parsec+Deskflow.

This doc is read-only evidence. No code changes proposed until the existing
PR queue lands.

## 1. What is real and reachable today

Verified by direct file inspection, not memory:

| Surface | File | Lines | Status |
|---|---|---|---|
| Capability descriptor enum | `crates/fabric-capability/src/descriptor.rs` | 277+ | ENUMERATED (compute/display/input/audio/accelerator/pcie/storage/network/topology) |
| Capability probe trait + POSIX impl | `crates/fabric-capability/src/probe.rs` | 330 | REAL — populates `ComputeCapabilities` for Linux+macOS |
| Frame transport codec enum | `crates/fabric-frame-transport/src/lib.rs:82` | — | `Codec` enum defined |
| Frame transport message types | `crates/fabric-frame-transport/src/lib.rs:231` | — | `FrameMessage` defined |
| Frame transport wire encode/parse | `crates/fabric-frame-transport/src/transport.rs:141/169` | — | REAL |
| Integration harness streamer | `crates/fabric-integration-tests/src/harness/frame_streamer.rs` | 213 | REAL — opens TCP, sends SessionInit + frames as JSON lines |
| Daemon wire server (TCP listener) | `crates/fabric-daemon/src/wire/mod.rs:30` | — | REAL — `run_wire_server(listener)` |
| Capabilities sub-structs (Compute/Display/Audio/...) | descriptor.rs | many | ENUMERATED |

## 2. What is NOT real

| Surface | Status | Block |
|---|---|---|
| `probe/`, `tests/` (non-empty subdirs) | EMPTY | No populated capability descriptors beyond Compute |
| macOS/Windows probe impls | DEFINED trait, NOT IMPLEMENTED | Trait returns UnsupportedProbe on non-POSIX |
| Bench harness comparing to Parsec+Deskflow | NONE | Reinvented from scratch |
| End-to-end "two daemons talking and moving bytes" | NO real round-trip | Frame streamer doc says: "wire server currently handles JSON line protocol and will return an UNKNOWN_TYPE error... expected: the test proves the transport types and wire encoding work over a real TCP connection. A future wire server enhancement will dispatch frame messages to enable true round-trip validation." |

## 3. What this means for a pilot claim

A pilot is honest when ALL three hold:
1. The system actually runs end-to-end on real hardware.
2. The same workload is run against the comparison system on the same hardware.
3. The methodology doc is reproducible.

Today (2026-10-08) we hold 0 of 3 for any capability. We hold part of 1 for the daemon's auth flow, which is orthogonal to Parsec+Deskflow.

## 4. Pilot slice that is honest to begin NOW

The smallest truthful thing: characterize the existing TCP transport
that already runs between daemons. We don't need new code for this.

Concretely:
- Start a daemon (or stub listener that accepts the wire protocol).
- Drive `stream_frames_between()` against it with N=100, 1k, 10k frames.
- Measure throughput, RTT distribution, dropped frames.
- Compare against a Parsec 8.0 and Deskflow 1.20 session on the same LAN.

This is NOT a feature ship. It's "what does our existing transport actually do
in numbers, and is it in the same order of magnitude as Parsec/Deskflow?"

If yes → claim "performance envelope comparable, feature parity TBD."
If no → honest "we have a measurable gap in [layer]."

Either result is publishable. Both are more honest than what we have now
(nothing).

## 5. Time estimate

| Scenario | Wall-clock | Honest if |
|---|---|---|
| **Characterize existing TCP transport (slice #1)** | 1-2 weeks | Only if existing tests actually compile and run, which I have not verified |
| **Add one real end-to-end capability (LAN video + HID on Mac/Linux)** | 4-8 weeks | Requires picking a codec (HEVC?), wiring probe + transport + encode, building a Linux-side daemon binary, integration test on real hardware |
| **Parsec+Deskflow reproducible comparison** | +1 week on top of slice | Requires identical hardware and same machine, both machines idle for baseline |
| **Claim "competitive on LAN latency"** | 4-8 weeks total | Slice #1 + slice #2 milestone; realistic for 2-3 engineers + this assistant |
| **Claim "competitive on WAN"** | 12-16 weeks | Need ICE/STUN/TURN chosen, plus measured packet-loss behavior |
| **Claim "competitive on KVMFR/IVSHMEM/PCIe"** | 16-24+ weeks | We do not own these transports; we wrap them. Requires vendor SDK access |

## 6. What is the right first PR if/when the queue lands

Slice #1 above. Specifically:
- Re-run existing `crates/fabric-integration-tests/src/harness/frame_streamer.rs`
  with N=10000 frames at 1280x720.
- Capture the StreamResult JSON.
- Document the result in `docs/research/<date>-transport-baseline.md`.
- Cross-reference `crates/fabric-frame-transport/benches/transport.rs`
  (already exists, 200 lines presumably) for sub-component numbers.

This is one lane, one PR, no architectural decisions. The result tells us
whether we need slice #2 urgently or whether the existing transport is
already competitive.

## 7. Blocker rules that still apply

- No merge to main without operator hook approval (currently 3 in queue).
- No new worktree, no `cargo clean` while disk <20Gi free (currently 51Gi).
- No fabrication of benchmarks or comparison numbers. If a number is not
  reproducible from a public commit, it does not ship.

## 8. What was done vs. what was deferred (final, 2026-10-09 01:04 PDT)

| Step | Status | Owner |
|---|---|---|
| Survey this doc (file/line evidence, no memory) | DONE | daisy (parent) |
| `cargo check -p fabric-integration-tests` (warm target, 1m23s) | DONE | daisy (parent) |
| Slice #1 test file written (155 lines, real) | DONE | poodle |
| Slice #1 15-row dataset (5 runs each N=100, 1000, 10000) | DONE | poodle |
| `cargo fmt --check` on the new file | NOT DONE | pending P2 lane |
| `cargo clippy -D warnings` on the new file | NOT DONE | pending P2 lane |
| Full `cargo test -p fabric-integration-tests` regression | NOT DONE | pending P2 lane |
| Mutation proof on TCP_NODELAY | NOT DONE | pending P2 lane |
| Methodology README | DONE | daisy (parent, content work) |
| Next-lane brief (P2 ready to run in one turn) | DONE | daisy (parent, content work) |
| Lane failure documentation (3 failures, sustained-down) | DONE | daisy (parent) |
| Commit + push on `research/transport-baseline-2026-10-08` | NOT DONE | pending P2 lane |
| Open PR for the research branch | DEFERRED | not needed; research, not feature |
| Parsec/Deskflow comparison | NOT STARTED | requires P2 to land + queue to land |

Three consecutive OpenCode Go endpoint failures (poodle / duckling /
sunflower). The standing pattern is now: **no more dispatches to
OpenCode Go without an explicit operator signal that the endpoint
is back.** This is the persistence threshold the operator policy
(2026-09-24) named.

See `01_LANE_STATUS.md` for the lane-failure log, and
`02_NEXT_LANE_BRIEF.md` for the copy-pasteable P2 brief.

## 9. Sign-off when complete

This doc is signed-off when either:
- Slice #1 has shipped with reproducible numbers, OR
- Operator chooses to defer pilot work in favor of finishing the queue.