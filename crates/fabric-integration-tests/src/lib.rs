//! Shared test helpers for Phenotype Fabric integration tests.

pub mod harness;

use chrono::Utc;
use fabric_capability::descriptor::{
    AudioCapabilities, Capabilities, ComputeCapabilities, DisplayCapabilities, DisplayInfo,
    InputCapabilities, StorageCapabilities, StorageDevice,
};
use fabric_capability::LocalityTier;
use fabric_daemon::config::{DatabaseConfig, DaemonConfig};
use fabric_daemon::coordinator::Coordinator;
use fabric_frame_transport::transport::encode_wire;
use fabric_frame_transport::{FrameHeader, MessageType};
use fabric_graph::model::{
    Edge, EdgeId, Intent, IntentId, IntentRequirements, Node, NodeId, Topology,
};
use std::sync::Arc;
use bytes::BytesMut;
use uuid::Uuid;

/// Create a temporary database-backed Coordinator for testing.
pub fn make_coordinator() -> (Arc<Coordinator>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let config = DaemonConfig {
        database: DatabaseConfig {
            path: db_path,
            ..Default::default()
        },
        ..Default::default()
    };
    let coord = Arc::new(Coordinator::new(config).unwrap());
    (coord, dir)
}

/// Build a 4-node linear topology: n1 --L1--> n2 --L3--> n3 --L6--> n4
pub fn build_4node_topology() -> Topology {
    let mut topo = Topology::new();
    topo.add_node(Node::new(NodeId::new("n1"), LocalityTier::L1SameNuma));
    topo.add_node(Node::new(
        NodeId::new("n2"),
        LocalityTier::L2CrossNumaShm,
    ));
    topo.add_node(Node::new(NodeId::new("n3"), LocalityTier::L6Lan));
    topo.add_node(Node::new(NodeId::new("n4"), LocalityTier::L7Wan));

    topo.add_edge(Edge::new(
        EdgeId::new("e1-2"),
        NodeId::new("n1"),
        NodeId::new("n2"),
        LocalityTier::L1SameNuma,
    ))
    .unwrap();
    topo.add_edge(Edge::new(
        EdgeId::new("e2-3"),
        NodeId::new("n2"),
        NodeId::new("n3"),
        LocalityTier::L3PcieP2P,
    ))
    .unwrap();
    topo.add_edge(Edge::new(
        EdgeId::new("e3-4"),
        NodeId::new("n3"),
        NodeId::new("n4"),
        LocalityTier::L6Lan,
    ))
    .unwrap();

    topo
}

/// Create a synthetic RGBA frame payload: width * height * 4 bytes.
pub fn make_rgba_payload(width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut data = Vec::with_capacity(pixel_count * 4);
    for i in 0..pixel_count {
        data.push(((i * 4) & 0xFF) as u8);
        data.push(((i * 4 + 1) & 0xFF) as u8);
        data.push(((i * 4 + 2) & 0xFF) as u8);
        data.push(255u8);
    }
    data
}

/// Encode a FrameData message through the wire format (header + payload).
pub fn encode_frame_wire(header: &FrameHeader, payload: &[u8]) -> BytesMut {
    let mut body = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
    header.encode(&mut body);
    body.extend_from_slice(payload);
    encode_wire(MessageType::FrameData, &body).unwrap()
}

/// Decode a wire FrameData message back into (FrameHeader, Bytes).
pub fn decode_frame_wire(wire: &BytesMut) -> (FrameHeader, bytes::Bytes) {
    let mut reader = bytes::Bytes::copy_from_slice(&wire[5..]);
    let header = FrameHeader::decode(&mut reader).unwrap();
    (header, reader)
}

/// Make a CapabilityDescriptor for a machine with optional display/audio.
pub fn make_descriptor(
    cores: u32,
    memory_gib: u64,
    has_audio: bool,
    has_display: bool,
) -> fabric_capability::descriptor::CapabilityDescriptor {
    let mut desc = fabric_capability::descriptor::CapabilityDescriptor {
        node_id: Uuid::now_v7(),
        epoch: 1,
        schema_version: "phenotype.fabric.capability_descriptor/1".to_string(),
        probed_at: Utc::now(),
        probe_version: "0.1.0".to_string(),
        topology_hash: "test-hash-e2e".to_string(),
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
    };

    if has_display {
        desc.capabilities.display = Some(DisplayCapabilities {
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
        desc.capabilities.input = Some(InputCapabilities {
            keyboards: vec!["AT keyboard".to_string()],
            mice: vec!["Logitech G Pro".to_string()],
            touchscreens: vec![],
            gamepads: vec![],
        });
    }

    desc
}

/// Make a minimal CheckerManifest.
pub fn make_manifest(cores: u32, memory_gib: u64) -> fabric_checker::CheckerManifest {
    fabric_checker::CheckerManifest {
        memory_bytes: memory_gib * 1024 * 1024 * 1024,
        cpu_cores: cores,
        storage_bytes: 1024 * 1024 * 1024,
        os_families: vec!["linux".to_string()],
        arches: vec!["x86_64".to_string()],
        audio: false,
        network_peers: vec![],
        headless: true,
        realtime_island: false,
    }
}

/// Make a default streaming intent.
pub fn make_intent() -> Intent {
    Intent {
        id: IntentId::new(),
        name: "streaming-e2e".to_string(),
        requirements: IntentRequirements::default(),
        preferred_node: None,
        min_trust: Default::default(),
        expires_at: None,
        tags: vec![],
    }
}
