//! Transport stage catalog for multi-hop routing.
//!
//! Defines the set of available transport/transcoding stages that can be
//! inserted between topology hops.

use crate::LocalityTier;
use serde::{Deserialize, Serialize};

/// Unique identifier for a transport stage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StageId(pub String);

/// A transport or transcoding stage that can be inserted in a route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportStage {
    /// Unique stage identifier.
    pub id: StageId,
    /// Human-readable name.
    pub name: String,
    /// Input media format.
    pub input_format: MediaFormat,
    /// Output media format.
    pub output_format: MediaFormat,
    /// Locality tier where this stage runs.
    pub locality_tier: LocalityTier,
    /// Cost model for this stage.
    pub cost: StageCost,
    /// Hardware/resource claims.
    pub claims: StageClaims,
    /// Adapter name (e.g. "hevc_encoder", "quic_transport").
    pub adapter: String,
}

/// Media format descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediaFormat {
    /// Raw video (uncompressed).
    RawVideo { width: u32, height: u32, fps: u32 },
    /// HEVC encoded video.
    Hevc { width: u32, height: u32, fps: u32 },
    /// AV1 encoded video.
    Av1 { width: u32, height: u32, fps: u32 },
    /// Raw PCM audio.
    PcmAudio { sample_rate: u32, channels: u8 },
    /// Opus encoded audio.
    OpusAudio { sample_rate: u32, channels: u8 },
    /// DMA buffer handle (GPU texture export).
    DmaBuffer,
    /// Opaque/any format (passthrough).
    Any,
}

/// Cost model for a single stage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StageCost {
    /// Setup cost in microseconds.
    pub setup_us: u64,
    /// Per-frame processing cost in microseconds.
    pub per_frame_us: f64,
    /// Memory requirement in bytes.
    pub memory_bytes: u64,
    /// Required bandwidth in Mbps.
    pub bandwidth_mbps: f64,
}

/// Hardware/resource claims for a stage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StageClaims {
    /// Required codec (if any).
    pub codec: Option<String>,
    /// Requires GPU encoder.
    pub gpu_encoder: bool,
    /// Requires GPU decoder.
    pub gpu_decoder: bool,
    /// Capable of running in RT island.
    pub rt_capable: bool,
}

/// Get the built-in transport stage catalog.
pub fn builtin_stages() -> Vec<TransportStage> {
    vec![
        TransportStage {
            id: StageId("identity".into()),
            name: "Identity (passthrough)".into(),
            input_format: MediaFormat::Any,
            output_format: MediaFormat::Any,
            locality_tier: LocalityTier::L0SameProcess,
            cost: StageCost {
                setup_us: 0,
                per_frame_us: 0.0,
                memory_bytes: 0,
                bandwidth_mbps: 0.0,
            },
            claims: StageClaims {
                rt_capable: true,
                ..Default::default()
            },
            adapter: "identity".into(),
        },
        TransportStage {
            id: StageId("shm_copy".into()),
            name: "Shared memory copy".into(),
            input_format: MediaFormat::Any,
            output_format: MediaFormat::Any,
            locality_tier: LocalityTier::L0SameProcess,
            cost: StageCost {
                setup_us: 10,
                per_frame_us: 5.0,
                memory_bytes: 0,
                bandwidth_mbps: 0.0,
            },
            claims: StageClaims {
                rt_capable: true,
                ..Default::default()
            },
            adapter: "shm_copy".into(),
        },
        TransportStage {
            id: StageId("hevc_encode".into()),
            name: "HEVC encoder".into(),
            input_format: MediaFormat::RawVideo {
                width: 1920,
                height: 1080,
                fps: 60,
            },
            output_format: MediaFormat::Hevc {
                width: 1920,
                height: 1080,
                fps: 60,
            },
            locality_tier: LocalityTier::L2CrossNumaShm,
            cost: StageCost {
                setup_us: 1000,
                per_frame_us: 2000.0,
                memory_bytes: 256 * 1024 * 1024,
                bandwidth_mbps: 50.0,
            },
            claims: StageClaims {
                codec: Some("hevc".into()),
                gpu_encoder: true,
                ..Default::default()
            },
            adapter: "hevc_encoder".into(),
        },
        TransportStage {
            id: StageId("hevc_decode".into()),
            name: "HEVC decoder".into(),
            input_format: MediaFormat::Hevc {
                width: 1920,
                height: 1080,
                fps: 60,
            },
            output_format: MediaFormat::RawVideo {
                width: 1920,
                height: 1080,
                fps: 60,
            },
            locality_tier: LocalityTier::L2CrossNumaShm,
            cost: StageCost {
                setup_us: 500,
                per_frame_us: 1000.0,
                memory_bytes: 128 * 1024 * 1024,
                bandwidth_mbps: 50.0,
            },
            claims: StageClaims {
                codec: Some("hevc".into()),
                gpu_decoder: true,
                ..Default::default()
            },
            adapter: "hevc_decoder".into(),
        },
        TransportStage {
            id: StageId("av1_encode".into()),
            name: "AV1 encoder".into(),
            input_format: MediaFormat::RawVideo {
                width: 1920,
                height: 1080,
                fps: 60,
            },
            output_format: MediaFormat::Av1 {
                width: 1920,
                height: 1080,
                fps: 60,
            },
            locality_tier: LocalityTier::L2CrossNumaShm,
            cost: StageCost {
                setup_us: 1500,
                per_frame_us: 3000.0,
                memory_bytes: 512 * 1024 * 1024,
                bandwidth_mbps: 30.0,
            },
            claims: StageClaims {
                codec: Some("av1".into()),
                gpu_encoder: true,
                ..Default::default()
            },
            adapter: "av1_encoder".into(),
        },
        TransportStage {
            id: StageId("av1_decode".into()),
            name: "AV1 decoder".into(),
            input_format: MediaFormat::Av1 {
                width: 1920,
                height: 1080,
                fps: 60,
            },
            output_format: MediaFormat::RawVideo {
                width: 1920,
                height: 1080,
                fps: 60,
            },
            locality_tier: LocalityTier::L2CrossNumaShm,
            cost: StageCost {
                setup_us: 800,
                per_frame_us: 1500.0,
                memory_bytes: 256 * 1024 * 1024,
                bandwidth_mbps: 30.0,
            },
            claims: StageClaims {
                codec: Some("av1".into()),
                gpu_decoder: true,
                ..Default::default()
            },
            adapter: "av1_decoder".into(),
        },
        TransportStage {
            id: StageId("quic_transport".into()),
            name: "QUIC transport".into(),
            input_format: MediaFormat::Any,
            output_format: MediaFormat::Any,
            locality_tier: LocalityTier::L6Lan,
            cost: StageCost {
                setup_us: 500,
                per_frame_us: 500.0,
                memory_bytes: 4 * 1024 * 1024,
                bandwidth_mbps: 100.0,
            },
            claims: StageClaims::default(),
            adapter: "quic_transport".into(),
        },
        TransportStage {
            id: StageId("tcp_transport".into()),
            name: "TCP transport".into(),
            input_format: MediaFormat::Any,
            output_format: MediaFormat::Any,
            locality_tier: LocalityTier::L6Lan,
            cost: StageCost {
                setup_us: 200,
                per_frame_us: 300.0,
                memory_bytes: 2 * 1024 * 1024,
                bandwidth_mbps: 100.0,
            },
            claims: StageClaims::default(),
            adapter: "tcp_transport".into(),
        },
        TransportStage {
            id: StageId("unix_socket".into()),
            name: "Unix domain socket".into(),
            input_format: MediaFormat::Any,
            output_format: MediaFormat::Any,
            locality_tier: LocalityTier::L5Loopback,
            cost: StageCost {
                setup_us: 50,
                per_frame_us: 10.0,
                memory_bytes: 0,
                bandwidth_mbps: 0.0,
            },
            claims: StageClaims {
                rt_capable: true,
                ..Default::default()
            },
            adapter: "unix_socket".into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_stages_non_empty() {
        let stages = builtin_stages();
        assert!(!stages.is_empty());
        assert!(stages.len() >= 8);
    }

    #[test]
    fn each_stage_has_unique_id() {
        let stages = builtin_stages();
        let ids: Vec<&StageId> = stages.iter().map(|s| &s.id).collect();
        let mut unique = ids.clone();
        unique.dedup();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn identity_stage_has_zero_cost() {
        let stages = builtin_stages();
        let identity = stages.iter().find(|s| s.id.0 == "identity").unwrap();
        assert_eq!(identity.cost.setup_us, 0);
        assert_eq!(identity.cost.per_frame_us, 0.0);
        assert!(identity.claims.rt_capable);
    }

    #[test]
    fn encode_stages_require_gpu() {
        let stages = builtin_stages();
        for stage in &stages {
            if stage.id.0.contains("encode") {
                assert!(
                    stage.claims.gpu_encoder || stage.claims.codec.is_some(),
                    "encode stage {} should require GPU or codec",
                    stage.id.0
                );
            }
        }
    }
}
