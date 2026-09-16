# 03 — Work Breakdown Structure: 10-Minute Task Breakdown

## Overview

```
[2-node harness] ████████████████░░░░  80% existing code
═══════════════════════════════════════════════════════
G0 identity    ] ████████████████ 100%  (repo exists, all crates exist)
G1 docs        ] ██████████████░░  70%  (ADRs exist, this design fills gap)
G2 quality     ] ████████████░░░░  60%  (existing tests pass, harness adds new)
G3 green       ] ██████████░░░░░░  50%  (cargo check works, harness needs building)
G4 pilot       ] ████░░░░░░░░░░░░  20%  (this design is the pilot plan)
G5 product     ] ████████░░░░░░░░  40%  (daemon exists, harness is proto-product)
G6 ecosystem   ] ██████░░░░░░░░░░  30%  (federation exists, needs 2-node proof)
═══════════════════════════════════════════════════════

Overall: 53% (weighted average)
```

## Task Queue

### Phase A: Harness Infrastructure (4 tasks, ~40 min)

| # | Task | Est | Depends On | Status |
|---|------|-----|------------|--------|
| A.1 | Create `harness/mod.rs` with `DaemonInstance` struct, `start_daemon()`, `wait_for_daemon()`, `stop_daemon()` | 10m | — | [ ] |
| A.2 | Create `harness/mod.rs` with `start_two_daemons()` helper (random ports, separate temp DBs, separate shutdown flags) | 10m | A.1 | [ ] |
| A.3 | Create `harness/federation.rs` with `exchange_and_merge()` and `assert_merged_topology()` helpers | 10m | A.1 | [ ] |
| A.4 | Create `harness/frame_streamer.rs` with `stream_frames_between()` helper (SessionInit→Ack, N frames with FrameAck, Ping/Pong) | 10m | A.1 | [ ] |

### Phase B: Core 2-Node Tests (5 tasks, ~50 min)

| # | Task | Est | Depends On | Status |
|---|------|-----|------------|--------|
| B.1 | Write `two_node_health_check` test: both daemons start, accept connections, report healthy | 10m | A.2 | [ ] |
| B.2 | Write `two_node_topology_exchange` test: B fetches A's topology via `sync_topology()`, verify snapshot fields | 10m | A.2, A.3 | [ ] |
| B.3 | Write `two_node_multihop_compile` test: compile route from node_a to node_b, verify 2-step plan | 10m | A.2 | [ ] |
| B.4 | Write `two_node_frame_streaming` test: stream 10 RGBA frames between daemons, verify pixel integrity and RTT | 10m | A.2, A.4 | [ ] |
| B.5 | Write `two_node_heartbeat_roundtrip` test: heartbeat works across both TCP connections | 10m | A.2 | [ ] |

### Phase C: Advanced Scenarios (5 tasks, ~50 min)

| # | Task | Est | Depends On | Status |
|---|------|-----|------------|--------|
| C.1 | Write `two_node_topology_merge_strategies` test: test MergeAll, LocalPrimary, PeerPrimary with 2-node setup | 10m | B.2 | [ ] |
| C.2 | Write `two_node_node_failure_detection` test: stop daemon A, verify B cannot reach A via `sync_topology()` | 10m | B.2 | [ ] |
| C.3 | Write `two_node_workspace_conflict` test: both nodes create workspaces claiming the same seat, verify conflict detection | 10m | A.2 | [ ] |
| C.4 | Write `two_node_surface_lease` test: create SurfaceLease on merged topology, verify Pending→Active transition | 10m | B.2 | [ ] |
| C.5 | Write `two_node_full_pipeline` test: topology→compile→exchange→stream→persist→health in one test | 10m | B.1-B.5 | [ ] |

### Phase D: Validation & Polish (3 tasks, ~30 min)

| # | Task | Est | Depends On | Status |
|---|------|-----|------------|--------|
| D.1 | Run full test suite: `cargo test -p fabric-integration-tests` — all existing + new tests must pass | 10m | C.1-C.5 | [ ] |
| D.2 | Line-count check on all new files: `wc -l` — each ≤350 lines (hard limit 500) | 10m | D.1 | [ ] |
| D.3 | Commit, push, update session docs with results | 10m | D.2 | [ ] |

## Dependency Graph

```
A.1 ──┬── A.2 ──┬── B.1
      │         ├── B.3
      │         ├── B.5
      │         ├── B.2 ──── A.3
      │         │      └──── A.4 ──── B.4
      │         │
      │         ├── C.1 (from B.2)
      │         ├── C.2 (from B.2)
      │         ├── C.3
      │         └── C.4 (from B.2)
      │
      └── A.3
      └── A.4

B.1 ──┐
B.2 ──┤
B.3 ──┤
B.4 ──┼── B.5 (all B's feed into C.5)
B.5 ──┘    │
           └── C.5 ──── D.1 ──── D.2 ──── D.3
```

## Estimated Total Effort

| Phase | Tasks | Time |
|-------|-------|------|
| A: Harness Infrastructure | 4 | 40 min |
| B: Core 2-Node Tests | 5 | 50 min |
| C: Advanced Scenarios | 5 | 50 min |
| D: Validation & Polish | 3 | 30 min |
| **Total** | **17** | **~170 min (2.8h)** |

## Critical Path

```
A.1 → A.2 → B.2 → C.5 → D.1 → D.2 → D.3
```

Estimated critical path duration: **70 min** (7 tasks × 10 min).

## What Exists Already (No Build Needed)

| Component | Crate | Location | Tests |
|-----------|-------|----------|-------|
| `DaemonInstance` (partial) | `fabric-integration-tests` | `src/lib.rs:make_coordinator()` | Used in existing tests |
| `send_and_receive()` | `fabric-integration-tests` | `tests/e2e_smoke.rs` | Used in 7 smoke tests |
| `wait_for_daemon()` | `fabric-integration-tests` | `tests/e2e_smoke.rs` | Used in 7 smoke tests |
| `sync_topology()` | `fabric-daemon` | `federation/discovery.rs` | Unit tested |
| `merge_topologies_from_json()` | `fabric-daemon` | `federation/merge.rs` | 3 strategy tests |
| `build_4node_topology()` | `fabric-integration-tests` | `src/lib.rs` | Used in 4 tests |
| `make_rgba_payload()` | `fabric-integration-tests` | `src/lib.rs` | Used in 6 tests |
| `encode_frame_wire()` | `fabric-integration-tests` | `src/lib.rs` | Used in 5 tests |
| `FrameSender`/`FrameReceiver` | `fabric-frame-transport` | `src/transport.rs` | Unit tested |

## What Gets Built

| New File | Lines (est.) | Purpose |
|----------|-------------|---------|
| `harness/mod.rs` | ~150 | Two-node infrastructure (DaemonInstance, start/stop helpers) |
| `harness/federation.rs` | ~80 | Topology exchange and merge verification |
| `harness/frame_streamer.rs` | ~120 | Simulated frame streaming between daemons |
| `harness/failover.rs` | ~60 | Node failure simulation |
| `tests/two_node.rs` | ~400 | All 2-node test scenarios |

**Total new code:** ~810 lines across 5 files. All under 400 lines.

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Wire server thread timing | Tests flake due to bind delay | `wait_for_daemon()` with retry loop (already proven) |
| Temp DB locking | Two SQLite DBs in same process | Each gets separate `tempfile::TempDir` — no shared state |
| Port conflict | Two servers on random ports | `TcpListener::bind("127.0.0.1:0")` guarantees unique ports |
| Test isolation | Previous test leaves threads | `stop_daemon()` joins thread; `Drop` on `AtomicBool` |
| Frame payload memory | 1080p RGBA = 8MB per frame | Use 64x48 (12KB) for harness tests; 1920x1080 only in stress tests |
