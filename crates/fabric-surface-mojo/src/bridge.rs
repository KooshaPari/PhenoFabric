//! FFI bridge types representing what a Mojo GPU backend would expose.
//!
//! Mojo's MLIR-based compilation pipeline compiles a single kernel definition
//! into multiple targets (CPU, GPU, custom accelerator). The types below model
//! the C FFI surface that a hypothetical `libmojo_surface.so` would export.
//!
//! # Mapping Mojo's compile-time targeting to Fabric locality tiers
//!
//! Mojo compiles kernels with a `target` parameter:
//!
//! - **`target = "cpu"`** — emits x86/ARM SIMD intrinsics. Maps to Fabric tiers
//!   L0SameProcess through L2CrossNumaShm where the kernel runs in-process on
//!   the same host.
//!
//! - **`target = "gpu"`** — emits CUDA/ROCm/Metal PTX via MLIR. Maps to L3PcieP2P
//!   through L5Loopback where the kernel runs on a GPU attached to the same host
//!   or a locally networked peer.
//!
//! - **`target = "remote"`** — serializes kernel args as a protobuf/FlatBuffer
//!   and ships them to a remote executor. Maps to L6Lan through L8Oob.
//!
//! The key insight is that Mojo's parametric compilation means you write ONE
//! kernel definition and the compiler produces the right native code for each
//! backend. Fabric's `DispatchPlan` (see [`crate::dispatch`]) selects which
//! compiled artifact to use at runtime based on the `LocalityTier` of the
//! active route step.

use serde::{Deserialize, Serialize};

/// Configuration for a Mojo surface backend instance.
///
/// Mirrors what a `mojo_surface_config_t` C struct would hold across the FFI boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojoSurfaceConfig {
    /// Target compute backend ("cpu", "gpu", "remote").
    pub target: String,
    /// Maximum concurrent kernel invocations.
    pub max_concurrent_kernels: u32,
    /// GPU device index (ignored for CPU/remote targets).
    pub gpu_device_index: u32,
    /// Shared memory pool size in bytes (CPU/L2 only).
    pub shm_pool_bytes: u64,
    /// Frame buffer width in pixels.
    pub frame_width: u32,
    /// Frame buffer height in pixels.
    pub frame_height: u32,
    /// Pixel format (e.g. "RGBA8", "NV12", "BGRA8").
    pub pixel_format: String,
}

impl Default for MojoSurfaceConfig {
    fn default() -> Self {
        Self {
            target: "cpu".into(),
            max_concurrent_kernels: 1,
            gpu_device_index: 0,
            shm_pool_bytes: 256 * 1024 * 1024, // 256 MiB
            frame_width: 1920,
            frame_height: 1080,
            pixel_format: "RGBA8".into(),
        }
    }
}

impl MojoSurfaceConfig {
    /// Returns true if this config targets a GPU backend.
    pub fn is_gpu(&self) -> bool {
        self.target == "gpu"
    }

    /// Returns true if this config targets CPU SIMD.
    pub fn is_cpu(&self) -> bool {
        self.target == "cpu"
    }

    /// Returns true if this config targets remote RPC.
    pub fn is_remote(&self) -> bool {
        self.target == "remote"
    }

    /// Estimated frame buffer size in bytes (RGBA8).
    pub fn estimated_frame_bytes(&self) -> u64 {
        (self.frame_width as u64) * (self.frame_height as u64) * 4
    }
}

/// Handle to a compiled Mojo kernel.
///
/// In a real FFI this would be a raw pointer to the Mojo runtime's kernel object.
/// Here we model it as an opaque ID plus metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojoKernel {
    /// Unique kernel identifier (assigned by the Mojo runtime).
    pub kernel_id: u64,
    /// Human-readable kernel name.
    pub name: String,
    /// Which backend this kernel was compiled for.
    pub backend: String,
    /// Local memory required per thread (bytes).
    pub local_mem_per_thread: u64,
    /// Shared memory required per workgroup (bytes).
    pub shared_mem_per_workgroup: u64,
    /// Number of threads per workgroup (GPU) or 1 (CPU).
    pub threads_per_workgroup: u32,
    /// Estimated FLOPS for this kernel on the target backend.
    pub estimated_flops: f64,
}

/// A frame buffer managed by the Mojo surface backend.
///
/// In a real FFI this would wrap a Mojo `Buffer` object. The key property
/// is that the buffer is allocated on the device matching the target backend:
/// - CPU: page-locked host memory (for DMA to GPU)
/// - GPU: device-local VRAM
/// - Remote: serialized byte vector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojoFrameBuffer {
    /// Unique buffer identifier.
    pub buffer_id: u64,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Pixel format.
    pub pixel_format: String,
    /// Whether the buffer is currently mapped to the host (CPU-accessible).
    pub host_mapped: bool,
    /// Device index the buffer resides on (-1 for host memory).
    pub device_index: i32,
    /// Host-mapped pixel data (populated for CPU kernels).
    #[serde(default)]
    pub host_mapped_data: Vec<u8>,
}

impl MojoFrameBuffer {
    /// Create a new frame buffer descriptor.
    pub fn new(
        buffer_id: u64,
        width: u32,
        height: u32,
        pixel_format: impl Into<String>,
        device_index: i32,
    ) -> Self {
        let format = pixel_format.into();
        let bytes_per_pixel = match format.as_str() {
            "RGBA8" | "BGRA8" => 4,
            "NV12" => 3,
            _ => 4,
        };
        let size = (width as u64) * (height as u64) * (bytes_per_pixel as u64);
        Self {
            buffer_id,
            size_bytes: size,
            width,
            height,
            pixel_format: format,
            host_mapped: device_index == -1,
            device_index,
            host_mapped_data: Vec::new(),
        }
    }
}

/// Errors that can occur in the Mojo surface FFI bridge.
#[derive(Debug, thiserror::Error)]
pub enum MojoSurfaceError {
    /// The Mojo runtime is not available (not installed or library load failed).
    #[error("Mojo runtime not available: {0}")]
    RuntimeUnavailable(String),

    /// Invalid configuration for the requested backend.
    #[error("Invalid config: {0}")]
    InvalidConfig(String),

    /// Kernel compilation failed.
    #[error("Kernel compilation failed: {0}")]
    KernelCompilationFailed(String),

    /// Buffer allocation failed (out of device memory).
    #[error("Buffer allocation failed: {0}")]
    BufferAllocationFailed(String),

    /// Kernel dispatch failed.
    #[error("Dispatch failed: {0}")]
    DispatchFailed(String),

    /// The requested device is not available.
    #[error("Device not available: index {device_index}")]
    DeviceNotAvailable { device_index: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let cfg = MojoSurfaceConfig::default();
        assert!(cfg.is_cpu());
        assert!(!cfg.is_gpu());
        assert!(!cfg.is_remote());
        assert_eq!(cfg.frame_width, 1920);
        assert_eq!(cfg.frame_height, 1080);
    }

    #[test]
    fn frame_buffer_size_rgba() {
        let buf = MojoFrameBuffer::new(1, 1920, 1080, "RGBA8", -1);
        assert_eq!(buf.size_bytes, 1920 * 1080 * 4);
        assert!(buf.host_mapped);
    }

    #[test]
    fn frame_buffer_device() {
        let buf = MojoFrameBuffer::new(2, 1280, 720, "RGBA8", 0);
        assert!(!buf.host_mapped);
        assert_eq!(buf.device_index, 0);
    }

    #[test]
    fn estimated_frame_bytes_matches() {
        let cfg = MojoSurfaceConfig::default();
        assert_eq!(cfg.estimated_frame_bytes(), 1920 * 1080 * 4);
    }
}
