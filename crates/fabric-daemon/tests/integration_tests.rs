//! Cross-crate integration tests for Phenotype Fabric.
//!
//! These tests span multiple crates and verify they work together correctly.
//! Run with: `cargo test -p fabric-daemon --test integration_tests`

use fabric_capability::descriptor::{Capabilities, ComputeCapabilities, Signature};
use fabric_capability::LocalityTier;
use fabric_checker::checker::check;
use fabric_checker::manifest::CheckerManifest;
use fabric_graph::builder::TopologyBuilder;
use fabric_graph::multihop::{builtin_stages, compile_multihop};
use fabric_graph::surface::{CaptureDirection, SurfaceProtocol, SurfaceSpec};
use fabric_graph::surface_ops;
use fabric_graph::{IntentBuilder, NodeId, Topology};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `CapabilityDescriptor` with the given compute resources.
///
/// Includes all fields needed to pass the checker's hard checks:
/// - epoch != 0
/// - supported schema version
/// - non-nil node_id
/// - non-empty topology_hash
/// - recent probed_at
fn base_descriptor(cores_logical: u32, memory_bytes: u64) -> fabric_capability::CapabilityDescriptor {
    fabric_capability::CapabilityDescriptor {
        node_id: Uuid::now_v7(),
        epoch: 1,
        schema_version: "phenotype.fabric.capability_descriptor/1".to_string(),
        probed_at: chrono::Utc::now(),
        probe_version: "0.1.0".to_string(),
        topology_hash: "test-hash-abc123".to_string(),
        capabilities: Capabilities {
            compute: Some(ComputeCapabilities {
                processor: "test-cpu".to_string(),
                cores_physical: cores_logical,
                cores_logical,
                numa_nodes: 1,
                cache: vec![],
                memory_bytes,
                memory_bandwidth_mbps: Some(50_000.0),
                hyperthread_pairs: vec![],
                tdp_watts: Some(65.0),
            }),
            accelerator: None,
            display: None,
            pcie: None,
            audio: None,
            input: None,
            storage: None,
            network: None,
            topology: None,
        },
        signatures: vec![Signature {
            key_id: "test-key-1".into(),
            alg: "ed25519".into(),
            sig: "test-sig".into(),
            signed_at: chrono::Utc::now(),
        }],
    }
}

/// Build a headless `CheckerManifest` with the given compute requirements.
fn headless_manifest(cpu_cores: u32, memory_bytes: u64) -> CheckerManifest {
    CheckerManifest {
        memory_bytes,
        cpu_cores,
        storage_bytes: 0,
        os_families: vec![],
        arches: vec![],
        audio: false,
        network_peers: vec![],
        headless: true,
        realtime_island: false,
    }
}

/// Build a 3-node linear topology: n1 --L1--> n2 --L6--> n3.
fn three_node_topology() -> Topology {
    TopologyBuilder::new()
        .with_name("integration-test")
        .add_simple_node("n1", LocalityTier::L1SameNuma)
        .add_simple_node("n2", LocalityTier::L1SameNuma)
        .add_simple_node("n3", LocalityTier::L6Lan)
        .connect("n1", "n2", LocalityTier::L1SameNuma)
        .connect("n2", "n3", LocalityTier::L6Lan)
        .build()
}

/// Build a 4-node topology with an alternative edge between n2 and n3.
///
/// Primary BFS path: n1→n2→n3→n4 (3 hops).
/// Alternative edge n2→n3 at a different tier allows fallback generation.
fn four_node_topology() -> Topology {
    use fabric_graph::model::{Edge, EdgeId};

    let mut topo = TopologyBuilder::new()
        .with_name("multihop-test")
        .add_simple_node("n1", LocalityTier::L1SameNuma)
        .add_simple_node("n2", LocalityTier::L1SameNuma)
        .add_simple_node("n3", LocalityTier::L2CrossNumaShm)
        .add_simple_node("n4", LocalityTier::L6Lan)
        .connect("n1", "n2", LocalityTier::L1SameNuma)
        .connect("n2", "n3", LocalityTier::L3PcieP2P)
        .connect("n3", "n4", LocalityTier::L6Lan)
        .build();

    // Add an alternative edge n2→n3 at a different tier for fallback generation.
    // This edge has a different EdgeId so the fallback algorithm can find it.
    let alt_edge = Edge::new(
        EdgeId::new("n2-n3-alt"),
        NodeId::new("n2"),
        NodeId::new("n3"),
        LocalityTier::L6Lan,
    );
    topo.add_edge(alt_edge).expect("add alternative edge");
    topo
}

/// Create a minimal SurfaceSpec for WebRtc.
fn web_rtc_surface_spec() -> SurfaceSpec {
    SurfaceSpec {
        name: "webrtc-primary".to_string(),
        protocol: SurfaceProtocol::WebRtc,
        capture: Some(CaptureDirection::Bidirectional),
        locality_floor: LocalityTier::L6Lan,
        refresh_hz: Some(60),
        audio_sample_rate_hz: Some(48000),
        requires_rt_island: false,
        strict_epoch_binding: false,
        min_host_trust: fabric_graph::model::TrustLevel::Untrusted,
        expires_at: None,
    }
}

// ===========================================================================
// Test 1: topology_compile_check_lease
// ===========================================================================

#[test]
fn topology_compile_check_lease() {
    // 1. Build a topology with 3 nodes connected by edges.
    let topology = three_node_topology();
    assert_eq!(topology.node_count(), 3);
    assert_eq!(topology.edge_count(), 2);

    // 2. Compile a multihop route from n1 to n3.
    let source = NodeId::new("n1");
    let destination = NodeId::new("n3");
    let intent = IntentBuilder::new()
        .name("integration-route")
        .build();
    let catalog = builtin_stages();

    let result =
        compile_multihop(&topology, &source, &destination, &intent, &catalog)
            .expect("multihop compilation should succeed");

    // Route should span 3 nodes (n1 -> n2 -> n3).
    assert_eq!(result.primary.steps.len(), 3);
    assert_eq!(result.primary.steps[0].node, NodeId::new("n1"));
    assert_eq!(result.primary.steps[1].node, NodeId::new("n2"));
    assert_eq!(result.primary.steps[2].node, NodeId::new("n3"));

    // 3. Create a CheckerManifest requiring 4 cores, 1 GiB memory.
    let manifest = headless_manifest(4, 1024 * 1024 * 1024);

    // 4. Create a CapabilityDescriptor with 8 cores, 16 GiB memory.
    let descriptor = base_descriptor(8, 16 * 1024 * 1024 * 1024);

    // 5. Run the checker — should be admissible.
    let decision = check(&descriptor, &manifest);
    assert!(
        decision.is_admissible(),
        "checker should admit: {:?}",
        decision
    );

    // 6. Create a SurfaceLease for a WebRtc surface.
    let spec = web_rtc_surface_spec();
    let mut lease = surface_ops::new_lease(spec).expect("new_lease should succeed");
    assert_eq!(lease.state, fabric_graph::surface::LeaseState::Pending);

    // 7. Bind the lease to the first route step and verify Active.
    let first_step = result.primary.steps[0].clone();
    surface_ops::bind(&mut lease, result.primary.id, first_step)
        .expect("bind should succeed");
    assert_eq!(lease.state, fabric_graph::surface::LeaseState::Active);
}

// ===========================================================================
// Test 2: checker_rejects_insufficient_resources
// ===========================================================================

#[test]
fn checker_rejects_insufficient_resources() {
    // 1. Create a CapabilityDescriptor with 2 cores, 8 GiB memory (memory
    //    is sufficient so the memory check passes first).
    let descriptor = base_descriptor(2, 8 * 1024 * 1024 * 1024);

    // 2. Create a CheckerManifest requiring 8 cores, 4 GiB memory.
    let manifest = headless_manifest(8, 4 * 1024 * 1024 * 1024);

    // 3. Run the checker — should Reject with CoresInsufficient.
    let decision = check(&descriptor, &manifest);
    match &decision {
        fabric_checker::Decision::Reject {
            reason_code, ..
        } => {
            assert_eq!(
                *reason_code,
                fabric_checker::ReasonCode::CoresInsufficient,
                "expected CoresInsufficient, got {:?}",
                reason_code
            );
        }
        other => panic!("expected Reject with CoresInsufficient, got {:?}", other),
    }
}

// ===========================================================================
// Test 3: checker_rejects_missing_audio
// ===========================================================================

#[test]
fn checker_rejects_missing_audio() {
    // 1. Create a CapabilityDescriptor with good compute but no audio.
    let descriptor = base_descriptor(8, 16 * 1024 * 1024 * 1024);

    // 2. Create a CheckerManifest with audio: true.
    let manifest = CheckerManifest {
        memory_bytes: 1024,
        cpu_cores: 1,
        storage_bytes: 0,
        os_families: vec![],
        arches: vec![],
        audio: true,
        network_peers: vec![],
        headless: true,
        realtime_island: false,
    };

    // 3. Run checker — should Reject (audio required but missing).
    let decision = check(&descriptor, &manifest);
    match &decision {
        fabric_checker::Decision::Reject {
            reason_code, ..
        } => {
            assert_eq!(
                *reason_code,
                fabric_checker::ReasonCode::CaptureRequiredButMissing,
                "expected CaptureRequiredButMissing, got {:?}",
                reason_code
            );
        }
        other => panic!("expected Reject with CaptureRequiredButMissing, got {:?}", other),
    }
}

// ===========================================================================
// Test 4: multihop_compile_full_topology
// ===========================================================================

#[test]
fn multihop_compile_full_topology() {
    // 1. Build topology: n1 --L1SameNuma--> n2 --L3CrossNumaPcie--> n3 --L6Lan--> n4
    let topology = four_node_topology();
    assert_eq!(topology.node_count(), 4);
    assert_eq!(topology.edge_count(), 4);

    // 2. Compile route n1 -> n4.
    let source = NodeId::new("n1");
    let destination = NodeId::new("n4");
    let intent = IntentBuilder::new()
        .name("full-topology-route")
        .build();
    let catalog = builtin_stages();

    let result =
        compile_multihop(&topology, &source, &destination, &intent, &catalog)
            .expect("multihop compilation should succeed");

    // 3. Verify route has 4 steps (one per node: n1 -> n2 -> n3 -> n4).
    assert_eq!(result.primary.steps.len(), 4);

    // 4. Verify cost is computed (non-negative).
    assert!(result.cost.latency_us >= 0.0, "latency should be non-negative");

    // 5. Verify fallbacks are generated.
    assert!(
        !result.fallbacks.is_empty(),
        "fallbacks should be generated for a 3-hop route"
    );

    // Verify transport stages were selected for each hop (3 hops = 3 edges).
    assert_eq!(result.stages_per_hop.len(), 3);
}

// ===========================================================================
// Test 5: daemon_probe_roundtrip
// ===========================================================================

#[test]
fn daemon_probe_roundtrip() {
    use fabric_daemon::config::DaemonConfig;
    use fabric_daemon::coordinator::Coordinator;
    use fabric_graph::model::{Edge, EdgeId, Node};

    let tmp = tempfile::tempdir().expect("tempdir");
    let db_path = tmp.path().join("test_coordinator.db");

    let config = DaemonConfig {
        database: fabric_daemon::config::DatabaseConfig {
            path: db_path,
            ..Default::default()
        },
        ..Default::default()
    };

    // 1. Create a Coordinator.
    let coordinator = Coordinator::new(config).expect("coordinator creation");

    // 2. Set a topology with 2 nodes and 1 edge.
    let mut topo = Topology::default();
    topo.meta.name = "probe-test".to_string();
    topo.add_node(Node::new(
        NodeId::new("host-a"),
        LocalityTier::L1SameNuma,
    ));
    topo.add_node(Node::new(
        NodeId::new("host-b"),
        LocalityTier::L6Lan,
    ));
    topo.add_edge(Edge::new(
        EdgeId::new("a-b"),
        NodeId::new("host-a"),
        NodeId::new("host-b"),
        LocalityTier::L6Lan,
    ))
    .expect("add edge");
    coordinator.set_topology(topo).expect("set topology");

    // 3. Call topology_snapshot() — verify JSON contains nodes, edges, epoch.
    let snapshot = coordinator.topology_snapshot();
    let parsed: serde_json::Value =
        serde_json::from_str(&snapshot).expect("parse topology snapshot");

    assert_eq!(parsed["type"], "probe_response");
    assert_eq!(parsed["status"], "ok");
    assert!(parsed["node_count"].as_u64().unwrap() >= 2);
    assert!(parsed["edge_count"].as_u64().unwrap() >= 1);
    assert!(parsed["topology_epoch"].as_u64().unwrap() > 0);
    assert!(parsed["nodes"].as_array().unwrap().len() >= 2);
    assert!(parsed["edges"].as_array().unwrap().len() >= 1);

    // 4. Call plans_snapshot() — verify empty routes initially.
    let plans = coordinator.plans_snapshot();
    let plans_parsed: serde_json::Value =
        serde_json::from_str(&plans).expect("parse plans snapshot");
    assert_eq!(plans_parsed["type"], "routes_response");
    assert!(plans_parsed["routes"].as_array().unwrap().is_empty());

    // 5. Call capabilities_snapshot() — verify empty caps initially.
    let caps = coordinator.capabilities_snapshot();
    let caps_parsed: serde_json::Value =
        serde_json::from_str(&caps).expect("parse caps snapshot");
    assert_eq!(caps_parsed["type"], "capabilities_response");
    assert!(caps_parsed["capabilities"]
        .as_array()
        .unwrap()
        .is_empty());
}

// ===========================================================================
// Test 6: surface_lease_web_rtc
// ===========================================================================

#[test]
fn surface_lease_web_rtc() {
    // 1. Build a topology.
    let topology = three_node_topology();

    // 2. Compile a route plan.
    let source = NodeId::new("n1");
    let destination = NodeId::new("n3");
    let intent = IntentBuilder::new()
        .name("webrtc-route")
        .build();
    let catalog = builtin_stages();

    let result =
        compile_multihop(&topology, &source, &destination, &intent, &catalog)
            .expect("multihop compilation");

    // 3. Create a SurfaceSpec for WebRtc protocol.
    let spec = web_rtc_surface_spec();
    assert_eq!(spec.protocol, SurfaceProtocol::WebRtc);

    // 4. Create a SurfaceLease.
    let mut lease =
        surface_ops::new_lease(spec.clone()).expect("new_lease should succeed");
    assert_eq!(
        lease.state,
        fabric_graph::surface::LeaseState::Pending
    );

    // 5. Bind and verify lease state is Active.
    let step = result.primary.steps[0].clone();
    surface_ops::bind(&mut lease, result.primary.id, step)
        .expect("bind should succeed");
    assert_eq!(lease.state, fabric_graph::surface::LeaseState::Active);

    // 6. Verify lease has correct protocol.
    assert_eq!(lease.spec.protocol, SurfaceProtocol::WebRtc);
    assert_eq!(lease.spec.name, "webrtc-primary");
}

// ===========================================================================
// Test 7: frame_transport_session_init (compile-only + roundtrip)
// ===========================================================================

#[test]
fn frame_transport_session_init() {
    use fabric_frame_transport::{Codec, SessionInit};

    // 1. Create a SessionInit message.
    let init = SessionInit {
        version: fabric_frame_transport::PROTOCOL_VERSION,
        preferred_codec: Codec::Hevc,
        width: 1920,
        height: 1080,
        target_fps: 60,
        max_latency_ms: 33,
        client_id: "test-client".to_string(),
    };

    // 2. Serialize it.
    let json = serde_json::to_string(&init).expect("serialize SessionInit");
    assert!(json.contains("\"Hevc\""));
    assert!(json.contains("1920"));
    assert!(json.contains("1080"));

    // 3. Verify it deserializes correctly.
    let deserialized: SessionInit =
        serde_json::from_str(&json).expect("deserialize SessionInit");
    assert_eq!(deserialized.version, fabric_frame_transport::PROTOCOL_VERSION);
    assert_eq!(deserialized.preferred_codec, Codec::Hevc);
    assert_eq!(deserialized.width, 1920);
    assert_eq!(deserialized.height, 1080);
    assert_eq!(deserialized.target_fps, 60);
    assert_eq!(deserialized.max_latency_ms, 33);
    assert_eq!(deserialized.client_id, "test-client");
}

// ===========================================================================
// Test 8: nvms_to_checker_flow
// ===========================================================================

#[test]
fn nvms_to_checker_flow() {
    use phenotype_nvms_adapter::phenotype_manifest::validate as validate_manifest;
    use phenotype_nvms_adapter::required_capabilities;

    // 1. Parse a minimal NVMS manifest JSON.
    let manifest_json = r#"{
        "app": {
            "name": "integration-test-app",
            "runtime": "python"
        },
        "infra": {
            "engine": "docker",
            "resources": {
                "cpu": 4,
                "memory": "2Gi"
            }
        },
        "network": {
            "ports": [8080]
        }
    }"#;

    let manifest =
        validate_manifest(manifest_json).expect("NVMS manifest should parse");

    // 2. Convert to RequiredCapabilities using phenotype-nvms-adapter.
    let req = required_capabilities(&manifest)
        .expect("required_capabilities should succeed");

    // Verify the mapping extracted the right values.
    assert_eq!(req.compute.cores_physical, 4);
    assert_eq!(
        req.compute.memory_bytes,
        2 * 1024 * 1024 * 1024 // 2 GiB
    );
    assert_eq!(req.network.interfaces.len(), 1);

    // Extract values before moving into the descriptor.
    let cores = req.compute.cores_physical;
    let memory = req.compute.memory_bytes;
    let has_audio = req.audio.is_some();

    // 3. Create a CapabilityDescriptor from the required capabilities.
    let descriptor = fabric_capability::CapabilityDescriptor {
        node_id: Uuid::now_v7(),
        epoch: 1,
        schema_version: "phenotype.fabric.capability_descriptor/1".to_string(),
        probed_at: chrono::Utc::now(),
        probe_version: "0.1.0".to_string(),
        topology_hash: "nvms-test-hash".to_string(),
        capabilities: Capabilities {
            compute: Some(req.compute),
            accelerator: None,
            display: None,
            pcie: None,
            audio: req.audio,
            input: None,
            storage: None,
            network: Some(req.network),
            topology: None,
        },
        signatures: vec![Signature {
            key_id: "nvms-test-key".into(),
            alg: "ed25519".into(),
            sig: "nvms-test-sig".into(),
            signed_at: chrono::Utc::now(),
        }],
    };

    // 4. Create a CheckerManifest that matches the descriptor.
    let checker_manifest = CheckerManifest {
        memory_bytes: memory,
        cpu_cores: cores,
        storage_bytes: 0,
        os_families: vec![],
        arches: vec![],
        audio: has_audio,
        network_peers: vec![],
        headless: true,
        realtime_island: false,
    };

    // 5. Run the checker — should be admissible.
    let decision = check(&descriptor, &checker_manifest);
    assert!(
        decision.is_admissible(),
        "checker should admit NVMS-derived descriptor: {:?}",
        decision
    );
}
