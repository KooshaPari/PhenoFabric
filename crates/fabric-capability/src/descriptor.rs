//! Capability descriptor types.
//!
//! The [`CapabilityDescriptor`] is the central data structure of PF-WP-010.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::locality::{CopyPath, LocalityTier};

/// The top-level capability descriptor for a Fabric node.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CapabilityDescriptor {
    pub node_id: Uuid,
    pub epoch: u64,
    pub schema_version: String,
    pub probed_at: chrono::DateTime<chrono::Utc>,
    pub probe_version: String,
    /// BLAKE3 hash of the topology subgraph (compute + accelerator + display + pcie + topology).
    pub topology_hash: String,
    pub capabilities: Capabilities,
    pub signatures: Vec<Signature>,
}

/// Discovered capabilities organized by category.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Capabilities {
    pub compute: Option<ComputeCapabilities>,
    pub accelerator: Option<AcceleratorCapabilities>,
    pub display: Option<DisplayCapabilities>,
    pub pcie: Option<PcieCapabilities>,
    pub audio: Option<AudioCapabilities>,
    pub input: Option<InputCapabilities>,
    pub storage: Option<StorageCapabilities>,
    pub network: Option<NetworkCapabilities>,
    pub topology: Option<TopologyCapabilities>,
}

// ---------------------------------------------------------------------------
// Sub-capability structs
// ---------------------------------------------------------------------------

/// Compute capabilities: CPU, cores, NUMA, cache, memory.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ComputeCapabilities {
    pub processor: String,
    pub cores_physical: u32,
    pub cores_logical: u32,
    pub numa_nodes: u32,
    pub cache: Vec<CacheInfo>,
    pub memory_bytes: u64,
    pub memory_bandwidth_mbps: Option<f64>,
    pub hyperthread_pairs: Vec<(u32, u32)>,
    pub tdp_watts: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CacheInfo {
    pub level: u8,
    pub size_bytes: u64,
    pub line_size_bytes: u32,
    pub cores_sharing: u32,
    pub numa_node: Option<u32>,
}

/// GPU, NPU, and hardware codec capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AcceleratorCapabilities {
    pub gpus: Vec<GpuInfo>,
    pub npu_present: bool,
    pub hardware_codecs: HardwareCodecMatrix,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GpuInfo {
    pub vendor: String,
    pub model: String,
    pub vram_bytes: Option<u64>,
    pub compute_capability: Option<String>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct HardwareCodecMatrix {
    pub av1_encode: bool,
    pub av1_decode: bool,
    pub h264_encode: bool,
    pub h264_decode: bool,
    pub h265_encode: bool,
    pub h265_decode: bool,
    pub vp9_encode: bool,
    pub vp9_decode: bool,
}

/// Connected displays.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DisplayCapabilities {
    pub displays: Vec<DisplayInfo>,
    pub wayland: bool,
    pub x11: bool,
    pub hdr_max_nits: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DisplayInfo {
    pub name: String,
    pub width_px: u32,
    pub height_px: u32,
    pub refresh_hz: Option<f64>,
    pub hdr: bool,
    pub edid_hash: Option<String>,
}

/// PCIe topology.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PcieCapabilities {
    pub p2p_supported: bool,
    pub p2p_devices: Vec<String>,
    pub devices: Vec<PcieDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PcieDevice {
    pub bus_id: String,
    pub vendor_id: u16,
    pub device_id: u16,
    pub lane_width: u8,
    pub generation: u8,
}

/// Audio devices.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AudioCapabilities {
    pub backend: String,
    pub sinks: Vec<AudioDevice>,
    pub sources: Vec<AudioDevice>,
    pub midi_ports: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AudioDevice {
    pub name: String,
    pub sample_rates: Vec<u32>,
    pub channels: u8,
}

/// Input devices.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct InputCapabilities {
    pub keyboards: Vec<String>,
    pub mice: Vec<String>,
    pub touchscreens: Vec<String>,
    pub gamepads: Vec<String>,
}

/// Storage devices.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StorageCapabilities {
    pub devices: Vec<StorageDevice>,
    pub network_mounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StorageDevice {
    pub path: String,
    pub size_bytes: u64,
    pub is_ssd: bool,
    pub read_iops_approx: Option<u64>,
    pub write_iops_approx: Option<u64>,
}

/// Network interfaces.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct NetworkCapabilities {
    pub interfaces: Vec<NetworkInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct NetworkInterface {
    pub name: String,
    pub mac_address: Option<String>,
    pub link_speed_mbps: Option<u64>,
    pub mtu: u32,
    pub rdma_capable: bool,
    pub zerocopy_capable: bool,
    pub rss_queues: u32,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
}

/// Topology graph edges.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TopologyCapabilities {
    pub edges: Vec<TopologyEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TopologyEdge {
    pub target_node_id: Uuid,
    pub locality_tier: LocalityTier,
    pub link_metrics: LinkMetrics,
}

/// Link metrics between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LinkMetrics {
    pub rtt_us: Percentiles<f64>,
    pub jitter_us: Option<f64>,
    pub loss_rate: f64,
    pub bandwidth_mbps: Percentiles<f64>,
    pub copy_paths: Vec<CopyPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Percentiles<T> {
    pub p50: T,
    pub p95: T,
    pub p99: T,
    pub max: T,
}

/// A cryptographic signature applied to a descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Signature {
    pub key_id: String,
    pub alg: String,
    pub sig: String,
    pub signed_at: chrono::DateTime<chrono::Utc>,
}

impl CapabilityDescriptor {
    /// Returns the canonical JSON bytes used as input to the signature.
    ///
    /// Serializes to `serde_json::Value` first, then strips the `signatures`
    /// field (a signature cannot sign itself), then emits compact bytes.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        let mut value = serde_json::to_value(self)
            .map_err(|e| Error::Serde(e.to_string()))?;
        if let Some(obj) = value.as_object_mut() {
            obj.remove("signatures");
        }
        serde_json::to_vec(&value)
            .map_err(|e| Error::Serde(e.to_string()))
    }

    /// Returns the BLAKE3 hash of the topology subgraph.
    ///
    /// Covers: compute + accelerator + display + pcie + topology.
    /// Excludes: audio, input, storage, network (can change without placement impact).
    pub fn topology_hash(&self) -> String {
        use blake3::Hasher;
        use serde::Serialize;

        fn hash_opt<T: Serialize + ?Sized>(opt: Option<&T>, hasher: &mut Hasher) {
            if let Some(c) = opt {
                let bytes = serde_json::to_vec(c).unwrap_or_default();
                hasher.update(&bytes);
            }
        }

        let mut hasher = Hasher::new();
        hash_opt(self.capabilities.compute.as_ref(), &mut hasher);
        hash_opt(self.capabilities.accelerator.as_ref(), &mut hasher);
        hash_opt(self.capabilities.display.as_ref(), &mut hasher);
        hash_opt(self.capabilities.pcie.as_ref(), &mut hasher);
        hash_opt(self.capabilities.topology.as_ref(), &mut hasher);
        hasher.finalize().to_hex().to_string()
    }

    /// Returns `true` if the descriptor has at least one signature
    /// from a key whose key_id is in `trusted_key_ids`.
    pub fn has_trusted_signature(&self, trusted_key_ids: &[String]) -> bool {
        self.signatures
            .iter()
            .any(|s| trusted_key_ids.contains(&s.key_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_descriptor() -> CapabilityDescriptor {
        CapabilityDescriptor {
            node_id: Uuid::now_v7(),
            epoch: 1,
            schema_version: "1.0.0".to_string(),
            probed_at: chrono::Utc::now(),
            probe_version: "0.1.0".to_string(),
            topology_hash: blake3::hash(&[]).to_hex().to_string(),
            capabilities: Capabilities::default(),
            signatures: vec![],
        }
    }

    #[test]
    fn descriptor_serializes() {
        let d = minimal_descriptor();
        let json = serde_json::to_string_pretty(&d).unwrap();
        assert!(json.contains("node_id"));
        assert!(json.contains("epoch"));
    }

    #[test]
    fn descriptor_roundtrip() {
        let d = minimal_descriptor();
        let json = serde_json::to_string(&d).unwrap();
        let roundtrip: CapabilityDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.epoch, d.epoch);
        assert_eq!(roundtrip.schema_version, d.schema_version);
        assert_eq!(roundtrip.node_id, d.node_id);
    }

    #[test]
    fn topology_hash_stable_for_empty() {
        // Two descriptors with same topology sections (both empty) → same hash
        let d1 = minimal_descriptor();
        let d2 = minimal_descriptor();
        assert_eq!(d1.topology_hash(), d2.topology_hash());
    }

    #[test]
    fn canonical_bytes_deterministic() {
        let d = minimal_descriptor();
        let b1 = d.canonical_bytes().unwrap();
        let b2 = d.canonical_bytes().unwrap();
        assert_eq!(b1, b2);
    }

    #[test]
    fn locality_tier_short_codes() {
        assert_eq!(LocalityTier::L0SameProcess.short_code(), "L0");
        assert_eq!(LocalityTier::L7Wan.short_code(), "L7");
        assert_eq!(LocalityTier::L8Oob.short_code(), "L8");
    }
}
