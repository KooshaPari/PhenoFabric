//! Tests: descriptor roundtrip (serialize → deserialize).
//!
//! Verifies that [`CapabilityDescriptor`] survives JSON serialization roundtrips
//! without data loss.

use fabric_capability::descriptor::{
    AcceleratorCapabilities, AudioCapabilities, AudioDevice, CapabilityDescriptor,
    ComputeCapabilities, DisplayCapabilities, DisplayInfo, HardwareCodecMatrix,
    InputCapabilities, NetworkCapabilities, NetworkInterface, PcieCapabilities,
    StorageCapabilities, StorageDevice,
};
use fabric_capability::locality::LocalityTier;
use uuid::Uuid;

fn minimal_descriptor() -> CapabilityDescriptor {
    CapabilityDescriptor {
        node_id: Uuid::now_v7(),
        epoch: 1,
        schema_version: "1.0.0".to_string(),
        probed_at: chrono::Utc::now(),
        probe_version: "0.1.0".to_string(),
        topology_hash: "a".repeat(64),
        capabilities: fabric_capability::descriptor::Capabilities::default(),
        signatures: vec![],
    }
}

fn full_descriptor() -> CapabilityDescriptor {
    let caps = fabric_capability::descriptor::Capabilities {
        compute: Some(ComputeCapabilities {
            processor: "AMD Ryzen 9 7950X".to_string(),
            cores_physical: 16,
            cores_logical: 32,
            numa_nodes: 1,
            cache: vec![],
            memory_bytes: 64 * 1024 * 1024 * 1024,
            memory_bandwidth_mbps: Some(50000.0),
            hyperthread_pairs: vec![],
            tdp_watts: Some(170.0),
        }),
        accelerator: Some(AcceleratorCapabilities {
            gpus: vec![],
            npu_present: false,
            hardware_codecs: HardwareCodecMatrix {
                av1_encode: true,
                av1_decode: true,
                h264_encode: true,
                h264_decode: true,
                h265_encode: true,
                h265_decode: true,
                vp9_encode: true,
                vp9_decode: true,
            },
        }),
        display: Some(DisplayCapabilities {
            displays: vec![DisplayInfo {
                name: "Samsung C27HG70".to_string(),
                width_px: 2560,
                height_px: 1440,
                refresh_hz: Some(144.0),
                hdr: true,
                edid_hash: None,
            }],
            wayland: false,
            x11: true,
            hdr_max_nits: Some(600),
        }),
        pcie: Some(PcieCapabilities {
            p2p_supported: true,
            p2p_devices: vec!["0000:01:00.0".to_string()],
            devices: vec![],
        }),
        audio: Some(AudioCapabilities {
            backend: "pipewire".to_string(),
            sinks: vec![AudioDevice {
                name: "HD Pro Webcam C920".to_string(),
                sample_rates: vec![48000],
                channels: 2,
            }],
            sources: vec![],
            midi_ports: 0,
        }),
        input: Some(InputCapabilities {
            keyboards: vec!["AT Translated Set 2 keyboard".to_string()],
            mice: vec!["Logitech G502".to_string()],
            touchscreens: vec![],
            gamepads: vec![],
        }),
        storage: Some(StorageCapabilities {
            devices: vec![StorageDevice {
                path: "/dev/nvme0n1".to_string(),
                size_bytes: 2 * 1024 * 1024 * 1024 * 1024,
                is_ssd: true,
                read_iops_approx: Some(700_000),
                write_iops_approx: Some(600_000),
            }],
            network_mounts: vec![],
        }),
        network: Some(NetworkCapabilities {
            interfaces: vec![NetworkInterface {
                name: "enp5s0".to_string(),
                mac_address: Some("aa:bb:cc:dd:ee:ff".to_string()),
                link_speed_mbps: Some(1000),
                mtu: 1500,
                rdma_capable: false,
                zerocopy_capable: false,
                rss_queues: 8,
                ipv4: Some("192.168.1.100".to_string()),
                ipv6: None,
            }],
        }),
        topology: None,
    };

    CapabilityDescriptor {
        node_id: Uuid::now_v7(),
        epoch: 42,
        schema_version: "1.0.0".to_string(),
        probed_at: chrono::Utc::now(),
        probe_version: "0.1.0".to_string(),
        topology_hash: "b".repeat(64),
        capabilities: caps,
        signatures: vec![],
    }
}

#[test]
fn test_minimal_roundtrip() {
    let d = minimal_descriptor();
    let json = serde_json::to_string(&d).unwrap();
    let round: CapabilityDescriptor = serde_json::from_str(&json).unwrap();
    assert_eq!(round.epoch, d.epoch);
    assert_eq!(round.schema_version, d.schema_version);
    assert_eq!(round.node_id, d.node_id);
}

#[test]
fn test_full_roundtrip() {
    let d = full_descriptor();
    let json = serde_json::to_string(&d).unwrap();
    let round: CapabilityDescriptor = serde_json::from_str(&json).unwrap();
    assert_eq!(round.epoch, d.epoch);

    let caps = round.capabilities;
    assert!(caps.compute.is_some());
    assert!(caps.accelerator.is_some());
    assert!(caps.display.is_some());
    assert!(caps.pcie.is_some());
    assert!(caps.audio.is_some());
    assert!(caps.input.is_some());
    assert!(caps.storage.is_some());
    assert!(caps.network.is_some());
}

#[test]
fn test_topology_hash_stable() {
    let d1 = minimal_descriptor();
    let d2 = minimal_descriptor();
    assert_eq!(d1.topology_hash(), d2.topology_hash());
}

#[test]
fn test_locality_tier_display() {
    assert_eq!(LocalityTier::L0SameProcess.short_code(), "L0");
    assert_eq!(LocalityTier::L7Wan.short_code(), "L7");
    assert_eq!(LocalityTier::L8Oob.short_code(), "L8");
    assert!(LocalityTier::L3PcieP2P.description().contains("PCIe"));
}

#[test]
fn test_locality_tier_serde() {
    let tier = LocalityTier::L4Rdma;
    let json = serde_json::to_string(&tier).unwrap();
    let round: LocalityTier = serde_json::from_str(&json).unwrap();
    assert_eq!(round, tier);
}

#[test]
fn test_epoch_increments() {
    let mut d = minimal_descriptor();
    assert_eq!(d.epoch, 1);
    d.epoch = 2;
    assert_eq!(d.epoch, 2);
}
