//! E2E streaming pipeline tests: daemon wire protocol, multihop compile + frame flow.
//!
//! Exercises the daemon message flow and the full pipeline:
//! - Daemon wire message sequence (topology/routes/lease/health/compile)
//! - Multihop compile + frame flow across a 4-node route
//! - Full E2E pipeline: topology -> compile -> check -> persist -> lease -> frame

use fabric_frame_transport::{Codec, FrameHeader};
use fabric_graph::model::NodeId;
use fabric_graph::multihop::builtin_stages;
use fabric_graph::surface::{CaptureDirection, LeaseState, SurfaceProtocol, SurfaceSpec};
use fabric_graph::surface_ops;

// Import shared helpers from the crate lib.
use fabric_integration_tests::{
    build_4node_topology, decode_frame_wire, encode_frame_wire, make_coordinator,
    make_descriptor, make_intent, make_manifest, make_rgba_payload,
};

// ===========================================================================
// Test 1: Daemon wire server message flow
// ===========================================================================

/// topology_request -> routes_request -> surface_lease sequence.
#[test]
fn daemon_wire_message_flow_topo_routes_lease() {
    let (coord, _dir) = make_coordinator();
    let topo = build_4node_topology();
    coord.set_topology(topo).unwrap();

    // -- topology_request --
    let topo_req = serde_json::json!({"type": "topology_request"});
    let resp = fabric_daemon::wire::protocol::process_message(&topo_req.to_string(), &coord).unwrap();
    let val: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(val["type"].as_str().unwrap(), "probe_response");
    assert!(val["topology_epoch"].is_number());
    assert_eq!(val["node_count"].as_u64().unwrap(), 4);
    assert_eq!(val["edge_count"].as_u64().unwrap(), 3);
    assert!(val["nodes"].as_array().map_or(false, |a| a.len() == 4));

    // -- routes_request --
    let routes_req = serde_json::json!({"type": "routes_request"});
    let resp = fabric_daemon::wire::protocol::process_message(&routes_req.to_string(), &coord).unwrap();
    let val: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(val["type"].as_str().unwrap(), "routes_response");
    assert!(val["routes"].is_array());

    // -- surface lease via surface_ops --
    let spec = SurfaceSpec {
        name: "streaming-display".to_string(),
        protocol: SurfaceProtocol::WebRtc,
        capture: Some(CaptureDirection::Bidirectional),
        locality_floor: fabric_capability::LocalityTier::L7Wan,
        refresh_hz: Some(60),
        audio_sample_rate_hz: Some(48000),
        requires_rt_island: false,
        strict_epoch_binding: false,
        min_host_trust: Default::default(),
        expires_at: None,
    };
    let lease = surface_ops::new_lease(spec).expect("surface_lease should succeed");
    assert_eq!(lease.state, LeaseState::Pending);
    assert!(matches!(lease.spec.protocol, SurfaceProtocol::WebRtc));
    assert_eq!(lease.spec.refresh_hz, Some(60));

    coord.insert_lease(lease);
    assert_eq!(coord.health().active_leases, 1);
}

// ===========================================================================
// Test 2: Daemon wire health -> topology -> compile sequence
// ===========================================================================

#[test]
fn daemon_wire_health_topology_compile_sequence() {
    let (coord, _dir) = make_coordinator();
    let topo = build_4node_topology();
    coord.set_topology(topo).unwrap();

    // health_check
    let resp = fabric_daemon::wire::protocol::process_message(
        r#"{"type":"health_check"}"#, &coord,
    ).unwrap();
    let val: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(val["status"].as_str().unwrap(), "healthy");

    // topology_request
    let resp = fabric_daemon::wire::protocol::process_message(
        r#"{"type":"topology_request"}"#, &coord,
    ).unwrap();
    let val: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(val["node_count"].as_u64().unwrap(), 4);

    // compile_request
    let resp = fabric_daemon::wire::protocol::process_message(
        r#"{"type":"compile_request","source":"n1","destination":"n4","intent_name":"stream"}"#,
        &coord,
    ).unwrap();
    let val: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(val["type"].as_str().unwrap(), "compile_response");
    assert_eq!(val["status"].as_str().unwrap(), "ok");
    let steps = val["plan"]["steps"].as_array().expect("plan should have steps");
    assert_eq!(steps.len(), 4, "4-node path -> 4 steps");
}

// ===========================================================================
// Test 3: Multihop compile + frame flow across 4-node route
// ===========================================================================

#[tokio::test]
async fn multihop_compile_and_frame_flow() {
    let topo = build_4node_topology();
    let stages = builtin_stages();
    let intent = make_intent();

    let result = fabric_graph::multihop::compile_multihop(
        &topo, &NodeId::new("n1"), &NodeId::new("n4"), &intent, &stages,
    )
    .expect("multihop compile should succeed");

    assert_eq!(result.primary.steps.len(), 4);
    assert_eq!(result.stages_per_hop.len(), 3);
    assert!(result.cost.latency_us >= 0.0);

    // Verify step actions.
    assert_eq!(result.primary.steps[0].action, "execute");
    assert_eq!(result.primary.steps[1].action, "route");
    assert_eq!(result.primary.steps[2].action, "route");
    assert_eq!(result.primary.steps[3].action, "receive");

    // Simulate frame flow: encode at each hop, decode, verify integrity.
    let width = 1280u32;
    let height = 720u32;
    let pixel_data = make_rgba_payload(width, height);
    let mut current_payload = pixel_data.clone();
    let mut frame_seq: u64 = 1;

    for (hop_idx, _step) in result.primary.steps.iter().enumerate() {
        let header = FrameHeader {
            seq: frame_seq,
            pts_us: frame_seq * 33_333,
            dts_us: frame_seq * 33_000,
            is_keyframe: hop_idx == 0,
            codec: Codec::Rgba,
            width, height,
            payload_len: current_payload.len() as u32,
            duration_us: 16_667,
        };

        let wire = encode_frame_wire(&header, &current_payload);
        let (decoded_header, decoded_payload) = decode_frame_wire(&wire);

        assert_eq!(decoded_header.seq, frame_seq, "hop {hop_idx}: seq");
        assert_eq!(decoded_header.width, width, "hop {hop_idx}: width");
        assert_eq!(
            &decoded_payload[..], &current_payload[..],
            "hop {hop_idx}: pixel data corrupted"
        );

        current_payload = decoded_payload.to_vec();
        frame_seq += 1;
    }

    // Final payload must match original.
    assert_eq!(current_payload, pixel_data);
}

// ===========================================================================
// Test 4: Full E2E pipeline — compile + check + persist + lease + frame
// ===========================================================================

#[test]
fn full_e2e_pipeline_compile_check_persist_frame() {
    let topo = build_4node_topology();

    // Compile multihop route n1 -> n4.
    let stages = builtin_stages();
    let intent = make_intent();
    let result = fabric_graph::multihop::compile_multihop(
        &topo, &NodeId::new("n1"), &NodeId::new("n4"), &intent, &stages,
    )
    .expect("multihop compile should succeed");
    assert_eq!(result.primary.steps.len(), 4);

    // Check admissibility.
    let descriptor = make_descriptor(8, 32, true, true);
    let manifest = make_manifest(4, 1);
    let decision = fabric_checker::check(&descriptor, &manifest);
    assert!(
        matches!(
            decision,
            fabric_checker::Decision::Admit | fabric_checker::Decision::AdmitWithNotes { .. }
        ),
        "checker should admit: got {decision:?}"
    );

    // Persist topology to SQLite.
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("e2e-streaming.db");
    let persist = fabric_persist::Persist::open(&db_path).expect("open persist");
    persist.save_topology(&topo).expect("save topology");
    let loaded = persist.load_topology().expect("load topology").expect("exists");
    assert_eq!(loaded.nodes.len(), 4);
    assert_eq!(loaded.edges.len(), 3);

    // Create surface lease.
    let spec = SurfaceSpec {
        name: "primary-display".to_string(),
        protocol: SurfaceProtocol::WebRtc,
        capture: Some(CaptureDirection::Bidirectional),
        locality_floor: fabric_capability::LocalityTier::L7Wan,
        refresh_hz: Some(60),
        audio_sample_rate_hz: Some(48000),
        requires_rt_island: false,
        strict_epoch_binding: false,
        min_host_trust: Default::default(),
        expires_at: None,
    };
    let lease = surface_ops::new_lease(spec).expect("surface_lease should succeed");
    assert_eq!(lease.state, LeaseState::Pending);

    // Encode a frame and verify roundtrip.
    let pixel_data = make_rgba_payload(1920, 1080);
    let header = FrameHeader {
        seq: 1, pts_us: 33_333, dts_us: 33_000,
        is_keyframe: true, codec: Codec::Rgba,
        width: 1920, height: 1080,
        payload_len: pixel_data.len() as u32,
        duration_us: 16_667,
    };
    let wire = encode_frame_wire(&header, &pixel_data);
    let (decoded_header, decoded_payload) = decode_frame_wire(&wire);
    assert_eq!(decoded_header.width, 1920);
    assert_eq!(decoded_header.height, 1080);
    assert_eq!(&decoded_payload[..], &pixel_data[..]);
}

// ===========================================================================
// Test 5: Daemon wire error handling
// ===========================================================================

#[test]
fn daemon_wire_invalid_json_and_unknown_type() {
    let (coord, _dir) = make_coordinator();

    // Invalid JSON
    let resp = fabric_daemon::wire::protocol::process_message("not json", &coord).unwrap();
    assert!(resp.contains("invalid_json"));

    // Unknown message type
    let resp = fabric_daemon::wire::protocol::process_message(
        r#"{"type":"foo_bar"}"#, &coord,
    ).unwrap();
    assert!(resp.contains("unknown_message"));

    // Compile request with missing source
    let resp = fabric_daemon::wire::protocol::process_message(
        r#"{"type":"compile_request","destination":"b"}"#, &coord,
    ).unwrap();
    assert!(resp.contains("missing_source"));

    // Compile request on empty topology
    let resp = fabric_daemon::wire::protocol::process_message(
        r#"{"type":"compile_request","source":"a","destination":"b"}"#, &coord,
    ).unwrap();
    assert!(resp.contains("compile_error") || resp.contains("compile_failed"));
}
