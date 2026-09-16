//! Two-node integration tests for the Fabric daemon.
//!
//! Each test starts two in-process daemons on random TCP ports, exercises
//! a specific federation or streaming scenario, and stops them cleanly.
//!
//! Run with: `cargo test -p fabric-integration-tests --test two_node`

use fabric_daemon::federation::MergeStrategy;
use fabric_frame_transport::Codec;
use fabric_graph::multihop::builtin_stages;
use fabric_graph::model::NodeId;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use fabric_integration_tests::harness::federation::{
    assert_federation_ids, assert_merged_topology, assert_min_epoch, exchange_and_merge,
};
use fabric_integration_tests::harness::frame_streamer::stream_frames_between;
use fabric_integration_tests::harness::{
    build_2node_topology, start_daemon, start_two_daemons, stop_daemon,
};
use fabric_integration_tests::make_intent;

// ---------------------------------------------------------------------------
// send_and_receive  (local helper, mirrors e2e_smoke.rs pattern)
// ---------------------------------------------------------------------------

/// Send a JSON line to the daemon and read the response line.
fn send_and_receive(addr: &str, message: &str) -> String {
    let mut stream = TcpStream::connect(addr).expect("connect to daemon");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .expect("set write timeout");

    write!(stream, "{message}\n").expect("write message");
    stream.flush().expect("flush");

    let reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut lines = reader.lines();
    lines
        .next()
        .expect("no response from daemon")
        .expect("io error reading response")
}

// ===========================================================================
// Test 1: Two-Node Health Check
// ===========================================================================

/// Both daemons start, accept TCP connections, and report healthy.
#[test]
fn two_node_health_check() {
    let (a, b) = start_two_daemons();

    // Query health over the real TCP wire protocol.
    let msg = r#"{"type":"health_check"}"#;
    let resp_a = send_and_receive(&a.addr, msg);
    let resp_b = send_and_receive(&b.addr, msg);

    let parsed_a: serde_json::Value = serde_json::from_str(&resp_a).expect("parse health_a");
    let parsed_b: serde_json::Value = serde_json::from_str(&resp_b).expect("parse health_b");

    assert_eq!(parsed_a["status"], "healthy");
    assert_eq!(parsed_b["status"], "healthy");

    // Verify uptime is reported (always >= 0).
    assert!(parsed_a["uptime_s"].as_u64().is_some());
    assert!(parsed_b["uptime_s"].as_u64().is_some());

    // Verify topology_epoch is reported.
    assert!(parsed_a["topology_epoch"].as_u64().is_some());
    assert!(parsed_b["topology_epoch"].as_u64().is_some());

    // Different ports.
    assert_ne!(a.addr, b.addr);

    stop_daemon(a);
    stop_daemon(b);
}

// ===========================================================================
// Test 2: Two-Node Topology Exchange
// ===========================================================================

/// B fetches A's topology via sync_topology(), verify snapshot fields and
/// that merging produces the expected node/edge counts.
#[test]
fn two_node_topology_exchange() {
    let (a, b) = start_two_daemons();

    // Give daemon_a a full 2-node topology with an edge.
    let topo = build_2node_topology();
    a.coordinator.set_topology(topo).expect("set topology for a");

    // B fetches A's topology over real TCP.
    let snapshot =
        fabric_daemon::federation::sync_topology(&a.addr).expect("sync_topology to a should succeed");

    // The snapshot should contain node data from A.
    assert!(
        snapshot.node_count >= 1,
        "expected at least 1 node in snapshot from A, got {}",
        snapshot.node_count,
    );

    // Merge: A (node_a + node_b + edge) + B (node_b) => 3 nodes (peer prefixed), 1 edge.
    // Peer node_b is prefixed with federation_id to avoid ID collisions.
    let merged = exchange_and_merge(&a, &b, &MergeStrategy::MergeAll);
    assert_merged_topology(&merged, 3, 1);
    // federation_ids comes from peer snapshots only (local nodes have empty federation_id).
    assert_federation_ids(&merged, 1);

    // max_epoch should be >= 0 (baseline check).
    assert_min_epoch(&merged, 0);

    stop_daemon(a);
    stop_daemon(b);
}

// ===========================================================================
// Test 3: Two-Node Multihop Compile
// ===========================================================================

/// Compile a route from node_a to node_b using the 2-node topology.
/// Verify the plan has 2 steps (execute + receive).
#[test]
fn two_node_multihop_compile() {
    let (a, _b) = start_two_daemons();

    // Set a 2-node topology on daemon A.
    let topo = build_2node_topology();
    a.coordinator
        .set_topology(topo.clone())
        .expect("set topology");

    let intent = make_intent();
    let catalog = builtin_stages();

    let result = fabric_graph::multihop::compile_multihop(
        &topo,
        &NodeId::new("node_a"),
        &NodeId::new("node_b"),
        &intent,
        &catalog,
    )
    .expect("compile_multihop should succeed");

    // 2-node direct path: 2 steps (execute at source, receive at destination).
    assert_eq!(
        result.primary.steps.len(),
        2,
        "expected 2 steps, got {}",
        result.primary.steps.len(),
    );

    // Steps should be execute + receive.
    assert_eq!(result.primary.steps[0].action, "execute");
    assert_eq!(result.primary.steps[1].action, "receive");

    // Nodes should match our topology.
    assert_eq!(result.primary.steps[0].node, NodeId::new("node_a"));
    assert_eq!(result.primary.steps[1].node, NodeId::new("node_b"));

    // Cost should be non-negative.
    assert!(
        result.cost.latency_us >= 0.0,
        "latency cost should be >= 0, got {}",
        result.cost.latency_us,
    );

    // Stages per hop should be populated.
    assert_eq!(
        result.stages_per_hop.len(),
        1,
        "1 hop (direct edge) should produce 1 set of stages"
    );

    stop_daemon(a);
}

// ===========================================================================
// Test 4: Two-Node Frame Streaming
// ===========================================================================

/// Stream synthetic RGBA frames from a client to daemon B's wire server.
/// Verify all frames are sent and the session completes without error.
#[test]
fn two_node_frame_streaming() {
    let (_a, b) = start_two_daemons();

    let width = 64;
    let height = 48;
    let frame_count = 10;

    let result = stream_frames_between(
        &b.addr,
        "client-test",
        width,
        height,
        frame_count,
        Codec::Rgba,
    )
    .expect("stream_frames_between should succeed");

    assert_eq!(
        result.frames_sent, frame_count,
        "expected {} frames sent, got {}",
        frame_count, result.frames_sent,
    );

    // All frames should have been acknowledged (local ack in current impl).
    assert_eq!(
        result.frames_acknowledged, frame_count,
        "expected {} frames acknowledged, got {}",
        frame_count, result.frames_acknowledged,
    );

    // On loopback, average RTT should be well under 10 ms (10,000 us).
    assert!(
        result.avg_rtt_us < 10_000,
        "avg RTT {} us exceeds 10 ms on loopback",
        result.avg_rtt_us,
    );

    // Session elapsed time should be non-zero (at least some wall time).
    assert!(
        result.elapsed > Duration::ZERO,
        "elapsed should be > 0"
    );

    stop_daemon(b);
}

// ===========================================================================
// Test 5: Two-Node Heartbeat Round-Trip
// ===========================================================================

/// Send heartbeats to both daemons over real TCP connections.
/// Verify each responds with heartbeat_ack.
#[test]
fn two_node_heartbeat_roundtrip() {
    let (a, b) = start_two_daemons();

    let msg = r#"{"type":"heartbeat"}"#;

    let resp_a = send_and_receive(&a.addr, msg);
    let resp_b = send_and_receive(&b.addr, msg);

    // Parse both responses.
    let parsed_a: serde_json::Value =
        serde_json::from_str(&resp_a).expect("parse heartbeat ack from a");
    let parsed_b: serde_json::Value =
        serde_json::from_str(&resp_b).expect("parse heartbeat ack from b");

    assert_eq!(parsed_a["type"], "heartbeat_ack");
    assert_eq!(parsed_a["status"], "ok");

    assert_eq!(parsed_b["type"], "heartbeat_ack");
    assert_eq!(parsed_b["status"], "ok");

    // Verify both daemons are still healthy after heartbeats.
    let health_a = send_and_receive(&a.addr, r#"{"type":"health_check"}"#);
    let health_b = send_and_receive(&b.addr, r#"{"type":"health_check"}"#);

    let ha: serde_json::Value = serde_json::from_str(&health_a).unwrap();
    let hb: serde_json::Value = serde_json::from_str(&health_b).unwrap();
    assert_eq!(ha["status"], "healthy");
    assert_eq!(hb["status"], "healthy");

    stop_daemon(a);
    stop_daemon(b);
}

// ===========================================================================
// Test 6: Merge Strategy Comparison
// ===========================================================================

/// Test all 3 merge strategies with overlapping node names.
///
/// Daemon A gets a 2-node topology (node_a → node_b). Daemon B already has
/// node_b from `start_two_daemons`. The strategies produce different results:
/// - MergeAll: B's node_b is prefixed → 3 nodes, 1 edge
/// - LocalPrimary: B's node_b has same base name → skipped → 2 nodes, 1 edge
/// - PeerPrimary: All peer nodes added (prefixed) → 3 nodes, 1 edge
#[test]
fn two_node_merge_strategy_comparison() {
    // --- MergeAll ---
    {
        let (a, b) = start_two_daemons();
        let topo = build_2node_topology();
        a.coordinator.set_topology(topo).expect("set topology for a");

        let merged = exchange_and_merge(&a, &b, &MergeStrategy::MergeAll);
        // A: {node_a, node_b, edge-a-b}, B: {node_b (prefixed)} => 3 nodes, 1 edge.
        assert_merged_topology(&merged, 3, 1);
        assert_federation_ids(&merged, 1);

        stop_daemon(a);
        stop_daemon(b);
    }

    // --- LocalPrimary ---
    {
        let (a, b) = start_two_daemons();
        let topo = build_2node_topology();
        a.coordinator.set_topology(topo).expect("set topology for a");

        let merged = exchange_and_merge(&a, &b, &MergeStrategy::LocalPrimary);
        // B's node_b has same base name as local node_b → peer version skipped.
        // A keeps its 2 nodes + 1 edge.
        assert_merged_topology(&merged, 2, 1);

        stop_daemon(a);
        stop_daemon(b);
    }

    // --- PeerPrimary ---
    {
        let (a, b) = start_two_daemons();
        let topo = build_2node_topology();
        a.coordinator.set_topology(topo).expect("set topology for a");

        let merged = exchange_and_merge(&a, &b, &MergeStrategy::PeerPrimary);
        // All peer nodes added (prefixed) alongside local nodes.
        // A: {node_a, node_b} + B: {B-prefixed node_b} = 3 nodes.
        assert_merged_topology(&merged, 3, 1);

        // Local node_b should still be present.
        let local_node_b = merged
            .nodes
            .iter()
            .any(|n| n.id == "node_b" && n.federation_id.is_empty());
        assert!(
            local_node_b,
            "local node_b should be retained in PeerPrimary merge"
        );

        stop_daemon(a);
        stop_daemon(b);
    }
}

// ===========================================================================
// Test 7: Node Failure Detection
// ===========================================================================

/// Mark a node as failed and verify topology is updated correctly.
///
/// Daemon A has a 2-node topology (node_a → node_b). After calling
/// `mark_node_failed("node_a")`, node_a and its edge are removed but
/// node_b persists.
#[test]
fn two_node_failure_detection() {
    let (a, b) = start_two_daemons();
    let topo = build_2node_topology();
    a.coordinator.set_topology(topo).expect("set topology for a");

    // Mark node_a as failed.
    let affected = a
        .coordinator
        .mark_node_failed(&NodeId::new("node_a"));
    assert_eq!(
        affected, 0,
        "no leases bound to node_a, so 0 should be affected"
    );

    // Verify node_a is removed from the topology snapshot.
    let snapshot = a.coordinator.topology_snapshot();
    let parsed: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
    let node_ids: Vec<&str> = parsed["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["id"].as_str())
        .collect();
    assert!(
        !node_ids.contains(&"node_a"),
        "node_a should be removed from topology after mark_node_failed"
    );
    assert!(
        node_ids.contains(&"node_b"),
        "node_b should still be present in topology"
    );

    // Verify the edge from node_a → node_b is also removed.
    let edges = parsed["edges"].as_array().unwrap();
    assert_eq!(
        edges.len(),
        0,
        "edge touching node_a should be removed; expected 0 edges, got {}",
        edges.len()
    );

    stop_daemon(a);
    stop_daemon(b);
}

// ===========================================================================
// Test 8: Surface Lease Lifecycle
// ===========================================================================

/// Insert a SurfaceLease into the coordinator and verify it appears
/// in the health check response.
#[test]
fn two_node_surface_lease_lifecycle() {
    let a = start_daemon("lease-test");

    // Insert an Active surface lease.
    let lease = fabric_graph::surface::SurfaceLease {
        handle: fabric_graph::surface::SurfaceHandle::new(),
        spec: fabric_graph::surface::SurfaceSpec {
            name: "test-surface".into(),
            protocol: fabric_graph::surface::SurfaceProtocol::Custom("test".into()),
            capture: None,
            locality_floor: fabric_graph::LocalityTier::L5Loopback,
            refresh_hz: None,
            audio_sample_rate_hz: None,
            requires_rt_island: false,
            strict_epoch_binding: false,
            min_host_trust: fabric_graph::TrustLevel::Untrusted,
            expires_at: None,
        },
        current: None,
        history: vec![],
        state: fabric_graph::surface::LeaseState::Active,
        exit_reason: None,
        created_at: chrono::Utc::now(),
        terminated_at: None,
    };
    a.coordinator.insert_lease(lease);

    // Verify via health check.
    let health = a.coordinator.health();
    assert_eq!(
        health.active_leases, 1,
        "expected 1 active lease after insert_lease"
    );

    // Also verify over the wire.
    let resp = send_and_receive(&a.addr, r#"{"type":"health_check"}"#);
    let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["status"], "healthy");
    assert_eq!(
        parsed["active_leases"], 1,
        "wire health check should report 1 active lease"
    );

    stop_daemon(a);
}

// ===========================================================================
// Test 9: Topology Epoch Tracking
// ===========================================================================

/// Verify that topology epochs are tracked correctly across set_topology
/// and merge operations.
#[test]
fn two_node_topology_epoch_tracking() {
    let (a, b) = start_two_daemons();

    // Both daemons start with epoch 1 after set_topology calls in
    // start_two_daemons (Topology::new()=0, add_node() bumps to 1).
    let epoch_a = a.coordinator.topology_epoch();
    let epoch_b = b.coordinator.topology_epoch();
    assert_eq!(epoch_a.0, 1, "initial epoch for daemon A should be 1");
    assert_eq!(epoch_b.0, 1, "initial epoch for daemon B should be 1");

    // Set a topology on A and manually bump its epoch.
    let mut topo_a = build_2node_topology();
    let _ = topo_a.epoch.bump(); // epoch = 4
    let _ = topo_a.epoch.bump(); // epoch = 5
    let _ = topo_a.epoch.bump(); // epoch = 6
    a.coordinator
        .set_topology(topo_a)
        .expect("set topology on A");

    let epoch_a_after = a.coordinator.topology_epoch();
    assert_eq!(
        epoch_a_after.0, 6,
        "daemon A epoch should be 6 after 3 bumps from build_2node_topology epoch=3"
    );

    // Set a different topology on B and bump its epoch higher.
    let mut topo_b = fabric_graph::Topology::new();
    topo_b.add_node(fabric_graph::model::Node::new(
        NodeId::new("node_x"),
        fabric_graph::LocalityTier::L7Wan,
    ));
    let _ = topo_b.epoch.bump(); // epoch = 1
    let _ = topo_b.epoch.bump(); // epoch = 2
    let _ = topo_b.epoch.bump(); // epoch = 3
    let _ = topo_b.epoch.bump(); // epoch = 4
    let _ = topo_b.epoch.bump(); // epoch = 5
    b.coordinator
        .set_topology(topo_b)
        .expect("set topology on B");

    let epoch_b_after = b.coordinator.topology_epoch();
    assert_eq!(
        epoch_b_after.0, 6,
        "daemon B epoch should be 6 after add_node + 5 bumps"
    );

    // Merge and verify max_epoch is the highest seen.
    let merged = exchange_and_merge(&a, &b, &MergeStrategy::MergeAll);
    assert_min_epoch(&merged, 6);

    // Verify the merged topology has nodes from both daemons.
    assert!(
        merged.nodes.len() >= 2,
        "merged topology should have at least 2 nodes"
    );

    stop_daemon(a);
    stop_daemon(b);
}

// ===========================================================================
// Test 10: Full Pipeline (Compile + Stream + Merge)
// ===========================================================================

/// End-to-end pipeline: compile a multihop route, stream frames, and merge
/// topologies on two daemons. Verifies all pieces work together.
#[test]
fn two_node_full_pipeline() {
    let (a, b) = start_two_daemons();

    // Give A a 2-node topology with an edge (node_a → node_b).
    let topo = build_2node_topology();
    a.coordinator
        .set_topology(topo.clone())
        .expect("set topology on A");

    // --- Step 1: Compile a multihop route on A ---
    let intent = make_intent();
    let catalog = builtin_stages();

    let compile_result = a
        .coordinator
        .compile_multihop(
            &NodeId::new("node_a"),
            &NodeId::new("node_b"),
            &intent,
            &catalog,
        )
        .expect("compile_multihop should succeed");

    // 2-node direct path: 2 steps (execute + receive).
    assert_eq!(
        compile_result.primary.steps.len(),
        2,
        "expected 2 steps, got {}",
        compile_result.primary.steps.len(),
    );

    // --- Step 2: Stream frames to B ---
    let frame_count = 5;
    let stream_result = stream_frames_between(
        &b.addr,
        "pipeline-client",
        64,
        48,
        frame_count,
        Codec::Rgba,
    )
    .expect("stream_frames_between should succeed");

    assert_eq!(
        stream_result.frames_sent, frame_count,
        "expected {} frames sent, got {}",
        frame_count, stream_result.frames_sent,
    );
    assert_eq!(
        stream_result.frames_acknowledged, frame_count,
        "expected {} frames acknowledged, got {}",
        frame_count, stream_result.frames_acknowledged,
    );

    // --- Step 3: Merge topologies ---
    let merged = exchange_and_merge(&a, &b, &MergeStrategy::MergeAll);
    // A: {node_a, node_b, edge-a-b} + B: {node_b (prefixed)} => 3 nodes, 1 edge.
    assert_merged_topology(&merged, 3, 1);

    stop_daemon(a);
    stop_daemon(b);
}
