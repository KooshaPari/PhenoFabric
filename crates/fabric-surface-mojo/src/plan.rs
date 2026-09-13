//! GPU dispatch plans with per-tier latency and memory estimates.
//!
//! A `GpuDispatchPlan` is the concrete output of the dispatch planner: it
//! contains the ordered steps a frame must traverse, the estimated latency
//! at each tier, and the memory requirements for the kernel.

use fabric_capability::LocalityTier;

use crate::ComputeBackend;

/// A single step in a GPU dispatch plan.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpuDispatchStep {
    /// Locality tier for this step.
    pub tier: LocalityTier,
    /// Compute backend to use.
    pub backend: ComputeBackend,
    /// Estimated latency in microseconds.
    pub latency_us: f64,
    /// Memory required for this step (bytes).
    pub memory_bytes: u64,
    /// Step description.
    pub description: String,
}

/// Complete GPU dispatch plan for a frame processing pipeline.
///
/// Contains all steps, total estimated latency, peak memory requirement,
/// and the frame dimensions the plan targets.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpuDispatchPlan {
    /// Ordered dispatch steps.
    pub steps: Vec<GpuDispatchStep>,
    /// Total estimated latency across all steps (microseconds).
    pub total_latency_us: f64,
    /// Peak memory requirement across all steps (bytes).
    pub peak_memory_bytes: u64,
    /// Frame width this plan targets.
    pub frame_width: u32,
    /// Frame height this plan targets.
    pub frame_height: u32,
}

impl GpuDispatchPlan {
    /// Create a dispatch plan for a single frame at a given locality tier.
    ///
    /// The plan includes:
    /// 1. Memory allocation step
    /// 2. Data transfer step (zero-copy for CPU, DMA for GPU, serialized for remote)
    /// 3. Kernel execution step
    /// 4. Result transfer step
    pub fn for_frame(
        tier: LocalityTier,
        width: u32,
        height: u32,
    ) -> Self {
        let pixel_count = (width as u64) * (height as u64);
        let frame_bytes = pixel_count * 4; // RGBA8

        let (backend, alloc_lat, transfer_lat, exec_lat, result_lat) = match tier {
            LocalityTier::L0SameProcess | LocalityTier::L1SameNuma | LocalityTier::L2CrossNumaShm => {
                (ComputeBackend::CpuSimd, 10.0, 0.0, 200.0, 0.0)
            }
            LocalityTier::L3PcieP2P | LocalityTier::L4Rdma | LocalityTier::L5Loopback => {
                (ComputeBackend::Gpu, 50.0, 200.0, 100.0, 150.0)
            }
            LocalityTier::L6Lan | LocalityTier::L7Wan | LocalityTier::L8Oob => {
                (ComputeBackend::RemoteRpc, 5.0, 5000.0, 1000.0, 5000.0)
            }
        };

        let steps = vec
![
            GpuDispatchStep {
                tier,
                backend,
                latency_us: alloc_lat,
                memory_bytes: frame_bytes,
                description: "Allocate frame buffer".into(),
            },
            GpuDispatchStep {
                tier,
                backend,
                latency_us: transfer_lat,
                memory_bytes: 0,
                description: "Transfer data to compute device".into(),
            },
            GpuDispatchStep {
                tier,
                backend,
                latency_us: exec_lat,
                memory_bytes: frame_bytes / 4, // working set smaller than full frame
                description: "Execute kernel".into(),
            },
            GpuDispatchStep {
                tier,
                backend,
                latency_us: result_lat,
                memory_bytes: 0,
                description: "Transfer results back".into(),
            },
        ];

        let total_latency_us = steps.iter().map(|s| s.latency_us).sum();
        let peak_memory_bytes = steps.iter().map(|s| s.memory_bytes).max().unwrap_or(0);

        Self {
            steps,
            total_latency_us,
            peak_memory_bytes,
            frame_width: width,
            frame_height: height,
        }
    }

    /// Estimated frames per second for this plan.
    pub fn estimated_fps(&self) -> f64 {
        if self.total_latency_us <= 0.0 {
            return 0.0;
        }
        1_000_000.0 / self.total_latency_us
    }

    /// Whether this plan meets a latency budget (microseconds).
    pub fn meets_latency_budget(&self, budget_us: f64) -> bool {
        self.total_latency_us <= budget_us
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_plan_low_latency() {
        let plan = GpuDispatchPlan::for_frame(LocalityTier::L0SameProcess, 1920, 1080);
        assert_eq!(plan.steps.len(), 4);
        assert!(plan.total_latency_us < 500.0, "CPU plan should be fast, got {}", plan.total_latency_us);
        assert!(plan.estimated_fps() > 60.0);
    }

    #[test]
    fn gpu_plan_moderate_latency() {
        let plan = GpuDispatchPlan::for_frame(LocalityTier::L3PcieP2P, 1920, 1080);
        assert!(plan.total_latency_us > 400.0);
        assert!(plan.total_latency_us < 1000.0);
    }

    #[test]
    fn remote_plan_high_latency() {
        let plan = GpuDispatchPlan::for_frame(LocalityTier::L7Wan, 1920, 1080);
        assert!(plan.total_latency_us > 10_000.0);
    }

    #[test]
    fn meets_budget() {
        let plan = GpuDispatchPlan::for_frame(LocalityTier::L0SameProcess, 1280, 720);
        assert!(plan.meets_latency_budget(10_000.0));
        assert!(!plan.meets_latency_budget(100.0));
    }

    #[test]
    fn peak_memory_scales_with_resolution() {
        let plan_720 = GpuDispatchPlan::for_frame(LocalityTier::L0SameProcess, 1280, 720);
        let plan_1080 = GpuDispatchPlan::for_frame(LocalityTier::L0SameProcess, 1920, 1080);
        assert!(plan_1080.peak_memory_bytes > plan_720.peak_memory_bytes);
    }

    #[test]
    fn frame_dimensions_preserved() {
        let plan = GpuDispatchPlan::for_frame(LocalityTier::L5Loopback, 3840, 2160);
        assert_eq!(plan.frame_width, 3840);
        assert_eq!(plan.frame_height, 2160);
    }
}
