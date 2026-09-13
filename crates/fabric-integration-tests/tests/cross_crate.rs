//! Cross-crate integration tests for Phenotype Fabric.
//!
//! Exercises the full flow: topology construction → multihop compilation →
//! checker admissibility → surface leasing → persistence → frame transport.

use bytes::Bytes;
use chrono::Utc;
use fabric_capability::descriptor::{
    AudioCapabilities, Capabilities, ComputeCapabilities, DisplayCapabilities, DisplayInfo,
    GpuInfo, AcceleratorCapabilities, HardwareCodecMatrix, InputCapabilities,
    StorageCapabilities, StorageDevice,
};
use fabric_capability::LocalityTier;
use fabric_frame_transport::{Codec, FrameHeader, FrameMessage, MessageType, SessionInit, PROTOCOL_VERSION};
use fabric_graph::model::{
    Edge, EdgeId, Intent, IntentId, IntentRequirements, LinkMetrics, Node, NodeId,
    Topology,
};
use fabric_graph::multihop::builtin_stages;
use fabric_graph::surface::{CaptureDirection, LeaseState, SurfaceProtocol, SurfaceSpec};
use fabric_graph::surface_ops;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_descriptor(cores: u32, memory_gib: u64, has_audio: bool) -> fabric_capability::descriptor::CapabilityDescriptor {
    fabric_capability::descriptor::CapabilityDescriptor {
        node_id: Uuid::now_v7(),
        epoch: 1,
        schema_version: "phenotype.fabric.capability_descriptor/1".to_string(),
        probed_at: Utc::now(),
        probe_version: "0.1.0".to_string(),
        topology_hash: "test-hash-abc123".to_string(),
        capabilities: Capabilities {
            compute: Some(ComputeCapabilities {
                processor: "test-cpu".to_string(),
                cores_physical: cores,
                cores_logical: cores,
                numa_nodes: 1,
                cache: vec![],
                memory_bytes: memory_gib * 1024 * 1024 * 1024,
                memory_bandwidth_mbps: None,
                hyperthread_pairs: vec![],
                tdp_watts: None,
            }),
            storage: Some(StorageCapabilities {
                devices: vec![StorageDevice {
                    path: "/dev/sda".to_string(),
                    size_bytes: 1024 * 1024 * 1024 * 1024,
                    is_ssd: true,
                    read_iops_approx: None,
                    write_iops_approx: None,
                }],
                network_mounts: vec![],
            }),
            audio: if has_audio {
                Some(AudioCapabilities {
                    backend: "alsa".to_string(),
                    sinks: vec![],
                    sources: vec![],
                    midi_ports: 0,
                })
            } else {
                None
            },
            ..Default::default()
        },
        signatures: vec![],
    }
}

fn make_manifest(cores: u32, memory_gib: u64, audio: bool) -> fabric_checker::CheckerManifest {
    fabric_checker::CheckerManifest {
        memory_bytes: memory_gib * 1024 * 1024 * 1024,
        cpu_cores: cores,
        storage_bytes: 1024 * 1024 * 1024,
        os_families: vec!["linux".to_string()],
        arches: vec!["x86_64".to_string()],
        audio,
        network_peers: vec![],
        headless: true,
        realtime_island: false,
    }
}

/// Build a linear topology: n1 --L1--> n2 --L3--> n3 --L6--> n4
fn build_linear_topology() -> Topology {
    let mut topo = Topology::new();

    let n1 = Node::new(NodeId::new("n1"), LocalityTier::L1SameNuma)
        .with_label("Host-1");
    let n2 = Node::new(NodeId::new("n2"), LocalityTier::L3PcieP2P)
        .with_label("Host-2");
    let n3 = Node::new(NodeId::new("n3"), LocalityTier::L6Lan)
        .with_label("Host-3");
    let n4 = Node::new(NodeId::new("n4"), LocalityTier::L7Wan)
        .with_label("Host-4");

    topo.add_node(n1);
    topo.add_node(n2);
    topo.add_node(n3);
    topo.add_node(n4);

    topo.add_edge(Edge::new(
        EdgeId::new("e1-2"),
        NodeId::new("n1"),
        NodeId::new("n2"),
        LocalityTier::L1SameNuma,
    )).unwrap();
    topo.add_edge(Edge::new(
        EdgeId::new("e2-3"),
        NodeId::new("n2"),
        NodeId::new("n3"),
        LocalityTier::L3PcieP2P,
    )).unwrap();
    topo.add_edge(Edge::new(
        EdgeId::new("e3-4"),
        NodeId::new("n3"),
        NodeId::new("n4"),
        LocalityTier::L6Lan,
    )).unwrap();

    topo
}

fn make_intent() -> Intent {
    Intent {
        id: IntentId::new(),
        name: "integration-test".to_string(),
        requirements: IntentRequirements::default(),
        preferred_node: None,
        min_trust: Default::default(),
        expires_at: None,
        tags: vec![],
    }
}

// ---------------------------------------------------------------------------
// Test 1: topology → compile → check → lease (full happy path)
// ---------------------------------------------------------------------------

#[test]
fn topology_compile_check_lease() {
    // 1. Build topology
    let topo = build_linear_topology();
    assert_eq!(topo.node_count(), 4);
    assert_eq!(topo.edge_count(), 3);

    // 2. Compile multihop route from n1 to n4
    let stages = builtin_stages();
    let intent = make_intent();
    let result = fabric_graph::multihop::compile_multihop(
        &topo,
        &NodeId::new("n1"),
        &NodeId::new("n4"),
        &intent,
        &stages,
    )
    .expect("multihop compile should succeed");

    // 4 nodes in path = 4 steps (execute on n1, route on n2, route on n3, receive on n4)
    assert_eq!(result.primary.steps.len(), 4);
    // 3 hops (edges) = 3 stages_per_hop
    assert_eq!(result.stages_per_hop.len(), 3);

    // 3. Run checker: descriptor with 8 cores, 16GiB vs manifest requiring 4 cores, 1GiB
    let descriptor = make_descriptor(8, 16, true);
    let manifest = make_manifest(4, 1, false);
    let decision = fabric_checker::check(&descriptor, &manifest);
    assert!(
        matches!(decision, fabric_checker::Decision::Admit | fabric_checker::Decision::AdmitWithNotes { .. }),
        "checker should admit: got {decision:?}"
    );

    // 4. Create a surface lease
    let spec = SurfaceSpec {
        name: "primary-display".to_string(),
        protocol: SurfaceProtocol::WebRtc,
        capture: None,
        locality_floor: LocalityTier::L7Wan,
        refresh_hz: Some(60),
        audio_sample_rate_hz: None,
        requires_rt_island: false,
        strict_epoch_binding: false,
        min_host_trust: Default::default(),
        expires_at: None,
    };
    let lease = surface_ops::new_lease(spec).expect("new_lease should succeed");
    assert_eq!(lease.state, LeaseState::Pending);
    assert!(matches!(lease.spec.protocol, SurfaceProtocol::WebRtc));
}

// ---------------------------------------------------------------------------
// Test 2: checker rejects insufficient resources
// ---------------------------------------------------------------------------

#[test]
fn checker_rejects_insufficient_resources() {
    let descriptor = make_descriptor(2, 1, true); // 2 cores, 1GiB
    let manifest = make_manifest(8, 4, false); // requires 8 cores, 4GiB

    let decision = fabric_checker::check(&descriptor, &manifest);
    assert!(
        matches!(decision, fabric_checker::Decision::Reject { .. }),
        "checker should reject: got {decision:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: checker rejects when audio required but missing
// ---------------------------------------------------------------------------

#[test]
fn checker_rejects_missing_audio() {
    let descriptor = make_descriptor(8, 16, false); // no audio
    let manifest = make_manifest(4, 1, true); // audio required

    let decision = fabric_checker::check(&descriptor, &manifest);
    assert!(
        matches!(decision, fabric_checker::Decision::Reject { .. }),
        "checker should reject (no audio): got {decision:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 4: multihop compile with full 4-node topology
// ---------------------------------------------------------------------------

#[test]
fn multihop_compile_full_topology() {
    let topo = build_linear_topology();
    let stages = builtin_stages();
    let intent = make_intent();

    let result = fabric_graph::multihop::compile_multihop(
        &topo,
        &NodeId::new("n1"),
        &NodeId::new("n4"),
        &intent,
        &stages,
    )
    .expect("compile should succeed");

    // 4 nodes in path = 4 steps (execute, route, route, receive)
    assert_eq!(result.primary.steps.len(), 4);
    // 3 edges = 3 stages
    assert_eq!(result.stages_per_hop.len(), 3);

    // Cost should be non-negative
    assert!(result.cost.latency_us >= 0.0);

    // Fallbacks may be empty in a linear topology (no alternate paths)
}

// ---------------------------------------------------------------------------
// Test 5: persistence save/load roundtrip
// ---------------------------------------------------------------------------

#[test]
fn persist_topology_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let topo = build_linear_topology();

    // Save
    let persist = fabric_persist::Persist::open(&db_path).expect("open persist");
    persist.save_topology(&topo).expect("save topology");

    // Load
    let loaded = persist.load_topology().expect("load topology").expect("topology should exist");
    assert_eq!(loaded.nodes.len(), 4);
    assert_eq!(loaded.edges.len(), 3);
    assert_eq!(loaded.epoch, topo.epoch);

    // Verify node labels
    assert_eq!(
        loaded.nodes.get(&NodeId::new("n1")).unwrap().label,
        Some("Host-1".to_string())
    );
    assert_eq!(
        loaded.nodes.get(&NodeId::new("n4")).unwrap().label,
        Some("Host-4".to_string())
    );
}

// ---------------------------------------------------------------------------
// Test 6: surface lease lifecycle
// ---------------------------------------------------------------------------

#[test]
fn surface_lease_web_rtc() {
    let spec = SurfaceSpec {
        name: "stream-display".to_string(),
        protocol: SurfaceProtocol::WebRtc,
        capture: Some(CaptureDirection::Bidirectional),
        locality_floor: LocalityTier::L7Wan,
        refresh_hz: Some(144),
        audio_sample_rate_hz: Some(48000),
        requires_rt_island: false,
        strict_epoch_binding: true,
        min_host_trust: Default::default(),
        expires_at: None,
    };

    let lease = surface_ops::new_lease(spec).expect("new lease");
    assert_eq!(lease.state, LeaseState::Pending);
    assert!(matches!(lease.spec.protocol, SurfaceProtocol::WebRtc));
    assert_eq!(lease.spec.refresh_hz, Some(144));
    assert_eq!(lease.spec.audio_sample_rate_hz, Some(48000));

    // Validate the spec
    assert!(lease.spec.validate().is_ok());
}

// ---------------------------------------------------------------------------
// Test 7: frame transport session_init roundtrip
// ---------------------------------------------------------------------------

#[test]
fn frame_transport_session_init_roundtrip() {
    let init = SessionInit {
        version: PROTOCOL_VERSION,
        preferred_codec: Codec::Hevc,
        width: 1920,
        height: 1080,
        target_fps: 60,
        max_latency_ms: 33,
        client_id: "integration-test-client".to_string(),
    };

    // Serialize to JSON (simulating wire encoding)
    let json = serde_json::to_vec(&init).unwrap();
    let decoded: SessionInit = serde_json::from_slice(&json).unwrap();

    assert_eq!(decoded.version, PROTOCOL_VERSION);
    assert_eq!(decoded.width, 1920);
    assert_eq!(decoded.height, 1080);
    assert_eq!(decoded.preferred_codec, Codec::Hevc);
    assert_eq!(decoded.client_id, "integration-test-client");
}

// ---------------------------------------------------------------------------
// Test 8: frame transport binary wire format
// ---------------------------------------------------------------------------

#[test]
fn frame_transport_binary_wire_format() {
    let header = FrameHeader {
        seq: 42,
        pts_us: 1_000_000,
        dts_us: 990_000,
        is_keyframe: true,
        codec: Codec::Av1,
        width: 2560,
        height: 1440,
        payload_len: 14,
        duration_us: 16_667,
    };

    // Encode header
    let mut body = bytes::BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + header.payload_len as usize);
    header.encode(&mut body);
    body.extend_from_slice(b"frame-payload!");

    // Verify wire format via encode_wire
    let wire = fabric_frame_transport::transport::encode_wire(
        MessageType::FrameData,
        &body,
    )
    .unwrap();

    // Wire should be: [4 bytes length LE] [1 byte type] [payload]
    let total_len = u32::from_le_bytes([wire[0], wire[1], wire[2], wire[3]]);
    assert_eq!(wire[4], MessageType::FrameData as u8);
    assert_eq!(total_len as usize, body.len());

    // Parse back
    let payload = Bytes::copy_from_slice(&wire[5..]);
    let msg = fabric_frame_transport::transport::parse_message(MessageType::FrameData, payload).unwrap();
    match msg {
        FrameMessage::FrameData { header: h, payload: p } => {
            assert_eq!(h.seq, 42);
            assert_eq!(h.width, 2560);
            assert!(h.is_keyframe);
            assert_eq!(&p[..], b"frame-payload!");
        }
        _ => panic!("expected FrameData"),
    }
}

// ---------------------------------------------------------------------------
// Test 9: checker admits headless when display required
// ---------------------------------------------------------------------------

#[test]
fn checker_display_check_headless_manifest() {
    let mut descriptor = make_descriptor(8, 16, true);
    descriptor.capabilities.display = Some(DisplayCapabilities {
        displays: vec![DisplayInfo {
            name: "eDP-1".to_string(),
            width_px: 1920,
            height_px: 1080,
            refresh_hz: Some(60.0),
            hdr: false,
            edid_hash: None,
        }],
        wayland: true,
        x11: false,
        hdr_max_nits: None,
    });
    descriptor.capabilities.input = Some(InputCapabilities {
        keyboards: vec!["AT Translated Set 2 keyboard".to_string()],
        mice: vec!["Logitech G Pro".to_string()],
        touchscreens: vec![],
        gamepads: vec![],
    });

    // Headless manifest — should skip display check
    let manifest = fabric_checker::CheckerManifest {
        memory_bytes: 1024 * 1024 * 1024,
        cpu_cores: 4,
        storage_bytes: 1024 * 1024 * 1024,
        os_families: vec![],
        arches: vec![],
        audio: false,
        network_peers: vec![],
        headless: true,
        realtime_island: false,
    };

    let decision = fabric_checker::check(&descriptor, &manifest);
    assert!(
        matches!(decision, fabric_checker::Decision::Admit | fabric_checker::Decision::AdmitWithNotes { .. }),
        "headless manifest should admit: got {decision:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 10: full end-to-end flow — probe → topology → compile → persist
// ---------------------------------------------------------------------------

#[test]
fn e2e_daemon_probe_topology_compile_persist() {
    // 1. Build topology from simulated probe results
    let mut topo = Topology::new();
    topo.meta.name = "e2e-test-topology".to_string();
    topo.meta.created_by = Some("integration-test".to_string());
    topo.meta.created_at = Some(Utc::now());

    let n1 = Node::new(NodeId::new("laptop"), LocalityTier::L1SameNuma)
        .with_label("MacBook Pro");
    let n2 = Node::new(NodeId::new("desktop"), LocalityTier::L2CrossNumaShm)
        .with_label("Desktop Workstation");

    topo.add_node(n1);
    topo.add_node(n2);

    topo.add_edge(Edge::new(
        EdgeId::new("lan-link"),
        NodeId::new("laptop"),
        NodeId::new("desktop"),
        LocalityTier::L2CrossNumaShm,
    ).with_metrics(LinkMetrics {
        latency_us: Some(100.0),
        bandwidth_bps: Some(10_000_000_000),
        packet_loss: Some(0.001),
        jitter_us: Some(10.0),
    }))
    .unwrap();

    assert_eq!(topo.epoch.0, 3); // 2 node adds + 1 edge add = epoch 3

    // 2. Compile route
    let stages = builtin_stages();
    let intent = make_intent();
    let result = fabric_graph::multihop::compile_multihop(
        &topo,
        &NodeId::new("laptop"),
        &NodeId::new("desktop"),
        &intent,
        &stages,
    )
    .expect("compile should succeed");

    // 2 nodes in path = 2 steps (execute on laptop, receive on desktop)
    assert_eq!(result.primary.steps.len(), 2);

    // 3. Check admissibility
    let descriptor = make_descriptor(8, 32, true);
    let manifest = make_manifest(4, 2, false);
    let decision = fabric_checker::check(&descriptor, &manifest);
    assert!(
        matches!(decision, fabric_checker::Decision::Admit | fabric_checker::Decision::AdmitWithNotes { .. }),
        "e2e check should admit: got {decision:?}"
    );

    // 4. Persist
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("e2e.db");
    let persist = fabric_persist::Persist::open(&db_path).expect("open persist");
    persist.save_topology(&topo).expect("save topology");

    let loaded = persist.load_topology().expect("load topology").expect("topology exists");
    assert_eq!(loaded.nodes.len(), 2);
    assert_eq!(loaded.edges.len(), 1);
    assert_eq!(loaded.meta.name, "e2e-test-topology");

    // 5. Create surface lease
    let spec = SurfaceSpec {
        name: "desktop-stream".to_string(),
        protocol: SurfaceProtocol::Vnc,
        capture: Some(CaptureDirection::Sink),
        locality_floor: LocalityTier::L3PcieP2P,
        refresh_hz: Some(60),
        audio_sample_rate_hz: None,
        requires_rt_island: false,
        strict_epoch_binding: false,
        min_host_trust: Default::default(),
        expires_at: None,
    };
    let lease = surface_ops::new_lease(spec).expect("new lease");
    assert_eq!(lease.state, LeaseState::Pending);
}
