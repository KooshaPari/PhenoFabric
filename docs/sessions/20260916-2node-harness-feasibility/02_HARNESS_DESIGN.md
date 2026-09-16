# 02 — Harness Design: Architecture, Transport, Simulation Strategy

## Architecture

The 2-node harness is a single Rust integration test binary (`fabric-integration-tests`)
that runs two daemon coordinators in-process, each on its own TCP port, connected via
loopback federation sync.

```
┌──────────────────────────────────────────────────────────┐
│  Single test process                                     │
│                                                          │
│  ┌─────────────────────┐    ┌─────────────────────┐      │
│  │  Coordinator A      │    │  Coordinator B      │      │
│  │  (temp SQLite DB)   │    │  (temp SQLite DB)   │      │
│  │  federation_id: "a" │    │  federation_id: "b" │      │
│  └─────────┬───────────┘    └─────────┬───────────┘      │
│            │                          │                  │
│  ┌─────────▼───────────┐    ┌─────────▼───────────┐      │
│  │  Wire Server A      │    │  Wire Server B      │      │
│  │  127.0.0.1:PORT_A   │    │  127.0.0.1:PORT_B   │      │
│  │  (TCP, blocking)    │    │  (TCP, blocking)    │      │
│  └─────────────────────┘    └─────────────────────┘      │
│            │                          │                  │
│            └──────── TCP loopback ────┘                  │
│                       │                                  │
│            ┌──────────▼──────────┐                       │
│            │  Federation Sync    │                       │
│            │  (sync_topology)    │                       │
│            └─────────────────────┘                       │
└──────────────────────────────────────────────────────────┘
```

### Key Design Decisions

**1. In-process, not multi-process**
Two coordinators run in the same test binary. Each gets its own `tempfile::TempDir`
with a separate SQLite database. Wire servers bind to `127.0.0.1:0` for random ports.
This avoids OS-level process management complexity while exercising the real TCP stack.

**2. Blocking wire server threads**
The existing wire server is blocking (`std::net::TcpListener`). Each server runs
on a `std::thread::spawn`. The test main thread coordinates setup/teardown via
`Arc<AtomicBool>` shutdown flags (already implemented in `Coordinator`).

**3. Federation via direct `sync_topology()` calls**
Instead of waiting for `spawn_sync_thread()` to fire (which has a configurable
interval), tests call `sync_topology(peer_addr)` directly for deterministic
timing. This exercises the real TCP client code path.

**4. Locality tier: L6 (LAN) for loopback peers**
On loopback, the locality tier is L5 (Loopback). For 2-node tests simulating
realistic LAN behavior, we use L6. The tier is set on the topology edges, not
on the transport itself.

## Harness Helpers (New Code)

### `harness/mod.rs` — Two-Node Test Infrastructure

```rust
/// A running daemon instance with coordinator and wire server.
pub struct DaemonInstance {
    pub coordinator: Arc<Coordinator>,
    pub addr: String,
    pub shutdown: Arc<AtomicBool>,
    pub _server_thread: JoinHandle<()>,
}

/// Start a daemon on a random port with a fresh temp DB.
pub fn start_daemon(federation_id: &str) -> DaemonInstance { ... }

/// Start two daemons wired as federation peers.
pub fn start_two_daemons() -> (DaemonInstance, DaemonInstance) { ... }

/// Wait until a daemon is accepting TCP connections.
pub fn wait_for_daemon(addr: &str, timeout: Duration) { ... }

/// Build a 2-node topology: node_a --L6--> node_b.
pub fn build_2node_topology() -> Topology { ... }

/// Build a 2-node topology with specific locality tiers.
pub fn build_2node_topology_with_tier(
    tier_a: LocalityTier,
    tier_b: LocalityTier,
    edge_tier: LocalityTier,
) -> Topology { ... }

/// Drop a daemon (trigger shutdown, join thread).
pub fn stop_daemon(instance: DaemonInstance) { ... }
```

### `harness/frame_streamer.rs` — Simulated Frame Flow

```rust
/// Simulate a frame streaming session between two daemons.
/// Sends SessionInit, verifies SessionAck, streams N frames with FrameAck.
pub fn stream_frames_between(
    server_addr: &str,   // daemon B (receiver)
    client_id: &str,
    width: u32, height: u32,
    frame_count: u32,
    codec: Codec,
) -> StreamResult { ... }
```

### `harness/federation.rs` — Topology Exchange

```rust
/// Fetch topology from daemon B, merge with daemon A's topology.
pub fn exchange_and_merge(
    daemon_a: &DaemonInstance,
    daemon_b: &DaemonInstance,
    strategy: &MergeStrategy,
) -> MergedTopology { ... }

/// Verify that merged topology contains nodes from both daemons.
pub fn assert_merged_topology(
    merged: &MergedTopology,
    expected_node_count: usize,
    expected_peer_count: usize,
) { ... }
```

### `harness/failover.rs` — Node Failure Simulation

```rust
/// Stop a daemon and verify the peer detects the failure.
/// Returns the time-to-detection in milliseconds.
pub fn simulate_node_failure(
    victim: DaemonInstance,
    observer_addr: &str,
    heartbeat_ttl_ms: u64,
) -> Duration { ... }
```

## Test Scenarios

### Scenario 1: Two-Node Health Check
**Validates:** Both daemons start, accept connections, report healthy.

```rust
#[test]
fn two_node_health_check() {
    let (a, b) = start_two_daemons();
    // Send health_check to both
    let resp_a = send_and_receive(&a.addr, r#"{"type":"health_check"}"#);
    let resp_b = send_and_receive(&b.addr, r#"{"type":"health_check"}"#);
    assert_eq!(resp_a.status, "healthy");
    assert_eq!(resp_b.status, "healthy");
}
```

### Scenario 2: Cross-Node Topology Exchange
**Validates:** Federation sync fetches and parses peer topology.

```rust
#[test]
fn two_node_topology_exchange() {
    let (a, b) = start_two_daemons();
    a.set_topology(build_2node_topology_node_a());
    b.set_topology(build_2node_topology_node_b());

    // B fetches A's topology
    let snapshot = sync_topology(&a.addr).unwrap();
    assert_eq!(snapshot.node_count, 1);
    assert_eq!(snapshot.source_addr, a.addr);

    // Merge
    let merged = exchange_and_merge(&a, &b, &MergeStrategy::MergeAll);
    assert_merged_topology(&merged, 2, 1);
}
```

### Scenario 3: Cross-Node Multihop Route Compilation
**Validates:** Route compiles across two different topologies.

```rust
#[test]
fn two_node_multihop_compile() {
    let (a, b) = start_two_daemons();
    let topo = build_2node_topology();  // A--L6-->B
    a.set_topology(topo.clone());

    let intent = make_intent();
    let stages = builtin_stages();
    let result = fabric_graph::multihop::compile_multihop(
        &topo, &NodeId::new("node_a"), &NodeId::new("node_b"),
        &intent, &stages,
    ).unwrap();

    assert_eq!(result.primary.steps.len(), 2);  // execute + receive
    assert!(result.cost.latency_us >= 0.0);
}
```

### Scenario 4: Cross-Node Frame Streaming
**Validates:** Frame data survives TCP roundtrip between two daemons.

```rust
#[test]
fn two_node_frame_streaming() {
    let (a, b) = start_two_daemons();
    let result = stream_frames_between(
        &b.addr, "client-a", 1920, 1080, 10, Codec::Rgba,
    );
    assert_eq!(result.frames_sent, 10);
    assert_eq!(result.frames_acknowledged, 10);
    assert!(result.avg_rtt_us < 10_000);  // <10ms on loopback
}
```

### Scenario 5: Heartbeat Round-Trip
**Validates:** Heartbeat/pong works across two TCP connections.

```rust
#[test]
fn two_node_heartbeat_roundtrip() {
    let (a, b) = start_two_daemons();
    let ping = r#"{"type":"heartbeat"}"#;
    let resp_a = send_and_receive(&a.addr, ping);
    let resp_b = send_and_receive(&b.addr, ping);
    assert!(resp_a.contains("heartbeat_ack"));
    assert!(resp_b.contains("heartbeat_ack"));
}
```

### Scenario 6: Node Failure Detection
**Validates:** Peer detects when other daemon stops.

```rust
#[test]
fn two_node_failure_detection() {
    let (a, b) = start_two_daemons();
    a.set_topology(build_2node_topology());

    // Stop daemon A
    stop_daemon(a);

    // Verify B can no longer reach A
    let result = sync_topology(&a_addr_after_stop);
    assert!(result.is_err());
}
```

### Scenario 7: Workspace Conflict Across Nodes
**Validates:** Two workspaces claiming the same seat are detected.

```rust
#[test]
fn two_node_workspace_conflict() {
    let (a, b) = start_two_daemons();
    // Both create workspaces claiming "display-1" seat
    // Merge topologies → detect conflict
    // Assert ConflictPair is returned
}
```

### Scenario 8: Compile-Stream-Persist Pipeline
**Validates:** Full lifecycle: topology → compile → frame → persist → health.

```rust
#[test]
fn two_node_full_pipeline() {
    let (a, b) = start_two_daemons();
    let topo = build_2node_topology();
    a.set_topology(topo.clone());

    // 1. Compile route
    let result = fabric_graph::multihop::compile_multihop(...);
    assert!(result.is_ok());

    // 2. Exchange topology
    let merged = exchange_and_merge(&a, &b, &MergeStrategy::MergeAll);
    assert_merged_topology(&merged, 2, 1);

    // 3. Stream frames
    let stream = stream_frames_between(&b.addr, ...);
    assert_eq!(stream.frames_acknowledged, stream.frames_sent);

    // 4. Verify health
    let health_a = send_and_receive(&a.addr, r#"{"type":"health_check"}"#);
    let health_b = send_and_receive(&b.addr, r#"{"type":"health_check"}"#);
    assert_eq!(health_a.status, "healthy");
    assert_eq!(health_b.status, "healthy");
}
```

## Transport Strategy

### Loopback TCP (Phase 1 — This Harness)

- Two TCP listeners on `127.0.0.1:0` (random ports).
- Federation `sync_topology()` connects via `TcpStream::connect(addr)`.
- Frame streaming uses `FrameSender`/`FrameReceiver` over the same TCP connection.
- All messages use the existing wire protocol (length-prefixed binary framing).
- No TLS (loopback doesn't need it).

### Same-Mac VM Route (Phase 2 — Future)

- Daemon A on Mac, daemon B inside a Linux VM.
- Federation over the VM's virtual network (e.g., OrbBridge `192.168.64.x`).
- Locality tier: L6 (LAN) or L7 (WAN) depending on VM config.
- Frame streaming tests real network latency and MTU behavior.
- Requires: VM setup script, IP discovery, firewall rules.

### Two-Machine LAN (Phase 3 — Dossier Target)

- Mac Mini (desk) and MacBook (laptop) on WiFi.
- Federation over LAN IP.
- Locality tier: L6 (LAN).
- Real mDNS discovery (future) or static peer config.
- Frame streaming over WiFi latency (~2-5ms RTT).
- Requires: Tailscale or manual IP config, cross-machine test runner.

## Inputs and Outputs to Test

### Inputs
| Input | Source | What to Test |
|-------|--------|-------------|
| Topology graph | `TopologyBuilder` | Node/edge creation, epoch management |
| Capability descriptor | `make_descriptor()` | Signing, schema validation |
| Intent | `Intent { requirements, preferred_node }` | Hard filter + soft scoring |
| Wire message | JSON over TCP | Parsing, validation, dispatch |
| Frame payload | RGBA pixel data | Encode/decode fidelity |
| Federation config | Peer address list | Sync interval, merge strategy |

### Outputs
| Output | Verified By | What to Assert |
|--------|------------|---------------|
| Health response | `serde_json::from_str` | status="healthy", uptime>0 |
| Topology snapshot | `topology_snapshot()` | node_count, edge_count, epoch |
| Route plan | `compile_multihop()` | steps, cost, fallbacks |
| Frame wire bytes | `encode_wire` + `parse_message` | Header fields, payload integrity |
| Federation snapshot | `sync_topology()` | Nodes match source topology |
| Merged topology | `merge_topologies_from_json()` | Node count = A + B, no ID collisions |
| Surface lease | `surface_ops::new_lease()` | State transitions (Pending→Active) |
| Workspace conflict | `find_conflicts()` | ConflictPair for shared seat |

## Error Scenarios

| Error | Trigger | Expected Behavior |
|-------|---------|-------------------|
| Peer unreachable | `sync_topology("127.0.0.1:99999")` | `FederationError::ConnectionFailed` |
| Invalid topology response | Malformed JSON from peer | `FederationError::ParseError` |
| Missing topology fields | Empty response body | Default values (epoch=0, node_count=0) |
| Frame too large | Payload > 16MB | `anyhow::bail!` in FrameSender |
| Unknown wire message type | type="foo_bar" | UNKNOWN_TYPE error response |
| Compile with no route | source/dest not connected | compile_error response |
| Workspace already exists | Duplicate workspace ID | `Error::Conflict` |
