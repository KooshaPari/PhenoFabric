# 01 — Codebase Assessment: What Exists vs What Needs Building

## Summary Table

| Component | Exists? | Tested? | 2-Node Ready? |
|-----------|---------|---------|---------------|
| Capability descriptors | Yes | Yes | Yes |
| Topology graph model | Yes | Yes (43 tests) | Yes |
| Locality tiers (L0-L8) | Yes | Yes | Yes |
| Route compiler (single-hop) | Yes | Yes | Yes |
| Multihop route compiler | Yes | Yes (24 tests) | Yes |
| Wire protocol (TCP framing) | Yes | Yes | Yes |
| Frame transport (RGBA/HEVC) | Yes | Yes | Yes |
| Daemon coordinator | Yes | Yes | Yes |
| Daemon wire server | Yes | Yes | Yes |
| Federation discovery | Yes | Yes (unit) | **Needs harness** |
| Federation merge | Yes | Yes (3 strategies) | **Needs harness** |
| Federation sync thread | Yes | No (spawned) | **Needs harness** |
| Workspace FSM | Yes | Yes | Partial (no seat conflict across nodes) |
| Route failover | Yes | Yes (pure fn) | **Needs harness** |
| Surface leases | Yes | Yes (in-process) | **Needs cross-node test** |
| WebRTC signaling | Partial | No | **Needs 2-node signaling** |
| Peer-to-peer frame streaming | No | No | **Needs build** |
| 2-node harness runner | No | No | **Needs build** |

## Detailed Assessment

### Already Built and Tested

**1. Capability Descriptors** (`fabric-capability`)
- `CapabilityDescriptor`: signed, versioned, self-describing node inventory.
- `Probe` trait for platform-specific discovery.
- `SigningKey`/`VerificationKey` with Ed25519.
- `TopologyEdge` with `LinkMetrics` (RTT percentiles, jitter, loss, bandwidth).
- Tests: roundtrip, property tests, signature verification, trust root chains.

**2. Topology Graph** (`fabric-graph`)
- `Topology` with `Node` (id, label, locality, capabilities, tags) and `Edge`.
- `TopologyEpoch` for staleness detection.
- `TopologyBuilder` for test fixture construction.
- `build_4node_topology()` in `fabric-integration-tests/src/lib.rs` — n1→n2→n3→n4.
- Pure functions: `negotiate`, `compile`, `plan_sequence`, `plan_parallel`.

**3. Multihop Route Compiler** (`fabric-graph::multihop`)
- BFS pathfinding through topology nodes.
- Per-hop transport stage selection (9 built-in stages: identity, shm_copy, encode/decode, quic/tcp/unix).
- `RouteCost` composite scoring.
- `validate_multihop()` for cycle detection, epoch matching.
- `generate_fallbacks()` for alternative routes.
- 24 unit tests.

**4. Wire Protocol** (`fabric-frame-transport`)
- Length-prefixed TCP binary framing: `[4 bytes len LE] [1 byte type] [payload]`.
- 8 message types: SessionInit, SessionAck, FrameData, FrameAck, Ping, Pong, Error, KeyFrameRequest.
- `FrameSender`/`FrameReceiver` over `TcpStream` (tokio async).
- Codec support: HEVC, AV1, NV12, RGBA.
- Frame header: 42 bytes fixed (seq, pts, dts, keyframe, codec, width, height, payload_len, duration).

**5. Daemon** (`fabric-daemon`)
- `Coordinator`: `Mutex<CoordinatorState>` holding topology, leases, plans.
- Wire server: blocking TCP with per-connection thread, JSON line protocol.
- Message types: heartbeat, health_check, probe_request, topology_request, routes_request, capabilities_request, webrtc_offer/answer/ice, compile_request, save_config.
- Auth middleware (JWT, OAuth).
- Federation module (see below).

**6. Federation** (`fabric-daemon::federation`)
- `sync_topology(addr)`: TCP client that sends `topology_request` and parses response.
- `merge_topologies_from_json()`: MergeAll, LocalPrimary, PeerPrimary strategies.
- `FederationState`: caches peer snapshots, tracks sync epoch.
- `spawn_sync_thread()`: background thread that periodically fetches from all configured peers.
- Unit tests for merge logic and state.

**7. Workspace FSM** (`fabric-workspace`)
- 5-state lifecycle: Unassigned → Assigned → Active → Completed/Failed/Cancelled.
- `SeatLease` FSM: Pending → Active → Released/Failed/Revoked.
- JSON file persistence via `WorkspaceStore`.
- Conflict detection across workspaces sharing a seat.

### Needs Building for 2-Node Harness

**1. Two-Instance Test Runner** (primary gap)
The existing `e2e_smoke.rs` starts ONE daemon on a random port. The 2-node harness needs TWO coordinators on separate ports, each with its own topology, wired as federation peers.

What's needed:
- A `start_two_daemons()` helper that:
  - Creates two `Coordinator` instances with separate temp DBs.
  - Binds two wire servers on different random ports.
  - Configures each daemon to know the other's address (federation peer list).
- A `wait_for_both()` helper that waits until both are accepting connections.

**2. Cross-Node Topology Exchange**
The federation `sync_topology()` function already does this. The harness needs to:
- Start daemon A, set topology with node A.
- Start daemon B, set topology with node B.
- Run `sync_topology` from B to A (or vice versa).
- Verify B received A's topology snapshot.
- Merge with `merge_topologies_from_json` and verify the merged view.

**3. Cross-Node Route Compilation**
After topology exchange, compile a route from node A to node B:
- Set up a 2-node topology (A at L6 LAN, B at L6 LAN) on daemon A.
- Compile multihop route from A to B.
- Verify the route has 2 steps (execute at A, receive at B).
- Verify the route includes the correct locality tier.

**4. Cross-Node Frame Flow**
Stream a frame from daemon A to daemon B:
- SessionInit from A → SessionAck from B.
- FrameData from A → FrameAck from B.
- Verify frame integrity (pixel data matches).
- Measure RTT via Ping/Pong.

**5. Workspace Conflict Across Nodes**
When both nodes claim the same seat:
- Create workspace W1 on node A with seat S.
- Create workspace W2 on node B with seat S.
- Verify conflict detection via federation merge.
- Test conflict resolution strategies.

**6. Heartbeat / Failover Across Nodes**
Simulate node failure:
- Node A sends heartbeats to node B.
- Stop node A (drop coordinator).
- Verify node B detects heartbeat miss (3 × ttl).
- Verify failover replan is triggered.
- Verify surface lease goes to Revoked.

### Transport Layer Assessment

| Transport | Currently Used | Latency | Reliability | 2-Node Viable? |
|-----------|---------------|---------|-------------|----------------|
| TCP (wire protocol) | Daemon wire server, federation sync | Low (LAN) | Reliable | **Primary choice** |
| JSON-over-TCP (spec 025) | Daemon messages | Low | Reliable | **Yes** |
| Frame binary (spec 025) | Frame transport | Low | Reliable | **Yes** |
| WebRTC (signaling) | Daemon wire (offer/answer/ice) | N/A | N/A | **Signaling only, no data plane yet** |
| UDP (STUN) | Network module | Very low | Unreliable | **For future P2P data plane** |
| Tailscale mesh | Network module | Low | Reliable | **For real multi-machine** |

**Recommendation for 2-node harness:** Use TCP on loopback (127.0.0.1) with two random ports. This exercises the real wire protocol without requiring actual network infrastructure. For "same-host VM route" (dossier phase 2), use 127.0.0.1 for one daemon and a container/VM IP for the other.

### What "2 Node" Means

**Phase 1 — Same Mac, Two Instances (Loopback)**
Two daemon processes on `127.0.0.1` with different ports. Federation sync over TCP.
Locality tier: L5 (Loopback) or L6 (LAN). This is what the harness builds.

**Phase 2 — Same Mac, One VM Route**
One daemon on the Mac, one inside a Linux VM (UTM/OrbStack). Federation over the VM's virtual network.
Locality tier: L6 (LAN) or L7 (WAN). Tests real cross-network behavior.

**Phase 3 — Desk-to-Laptop**
Mac Mini (desk) and MacBook (laptop) on the same WiFi/LAN. Federation over LAN.
Locality tier: L6 (LAN). The dossier's primary scenario.

Phase 1 is what we build now. It validates all the code paths that Phases 2 and 3 exercise, just without real network latency and topology diversity.
