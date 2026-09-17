//! Two-node integration tests for the Fabric daemon (advanced).
//!
//! Tests 6-10 cover merge strategies, failure detection, surface leases,
//! epoch tracking, and the full pipeline (compile + stream + merge).
//!
//! Core tests (1-5) are in `two_node.rs`.
//!
//! Run all: `cargo test -p fabric-integration-tests --test two_node --test two_node_advanced`

use fabric_daemon::federation::MergeStrategy;
use fabric_frame_transport::Codec;
use fabric_graph::model::NodeId;
use fabric_graph::multihop::builtin_stages;

use fabric_integration_tests::harness::federation::{
    assert_federation_ids, assert_merged_topology, assert_min_epoch, exchange_and_merge,
};
use fabric_integration_tests::harness::frame_streamer::stream_frames_between;
use fabric_integration_tests::harness::{
    build_2node_topology, send_and_receive, start_daemon, start_two_daemons, stop_daemon,
};
use fabric_integration_tests::make_intent;

// ===========================================================================
// Test 6: Merge Strategy Comparison
// ===========================================================================

/// Test all 3 merge strategies with overlapping node names.
///
/// Daemon A gets a 2-node topology (node_a -> node_b). Daemon B already has
/// node_b from `start_two_daemons`. The strategies produce different results:
/// - MergeAll: B's node_b is prefixed -> 3 nodes, 1 edge
/// - LocalPrimary: B's node_b has same base name -> skipped -> 2 nodes, 1 edge
/// - PeerPrimary: All peer nodes added (prefixed) -> 3 nodes, 1 edge
#[test]
fn two_node_merge_strategy_comparison() {
    // --- MergeAll ---
    {
        let (a, b) = start_two_daemons();
        let topo = build_2node_topology();
        a.coordinator
            .set_topology(topo)
            .expect("set topology for a");

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
        a.coordinator
            .set_topology(topo)
            .expect("set topology for a");

        let merged = exchange_and_merge(&a, &b, &MergeStrategy::LocalPrimary);
        // B's node_b has same base name as local node_b -> peer version skipped.
        // A keeps its 2 nodes + 1 edge.
        assert_merged_topology(&merged, 2, 1);

        stop_daemon(a);
        stop_daemon(b);
    }

    // --- PeerPrimary ---
    {
        let (a, b) = start_two_daemons();
        let topo = build_2node_topology();
        a.coordinator
            .set_topology(topo)
            .expect("set topology for a");

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
/// Daemon A has a 2-node topology (node_a -> node_b). After calling
/// `mark_node_failed("node_a")`, node_a and its edge are removed but
/// node_b persists.
#[test]
fn two_node_failure_detection() {
    let (a, b) = start_two_daemons();
    let topo = build_2node_topology();
    a.coordinator
        .set_topology(topo)
        .expect("set topology for a");

    // Mark node_a as failed.
    let affected = a.coordinator.mark_node_failed(&NodeId::new("node_a"));
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

    // Verify the edge from node_a -> node_b is also removed.
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

    // Give A a 2-node topology with an edge (node_a -> node_b).
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
    let stream_result =
        stream_frames_between(&b.addr, "pipeline-client", 64, 48, frame_count, Codec::Rgba)
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
