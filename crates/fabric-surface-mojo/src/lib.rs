//! `fabric-surface-mojo` — GPU compute surface backend with Mojo FFI bridge.
//!
//! This crate demonstrates how Mojo could be used as a GPU compute surface
//! backend for Phenotype Fabric. Since Mojo is pre-1.0 and may not be installed,
//! we model the FFI boundary as Rust types and provide a CPU fallback.
//!
//! # Architecture
//!
//! Mojo uses MLIR-based compilation to target CPUs, GPUs, and custom accelerators
//! from a single kernel definition. This maps directly onto Fabric's locality tiers:
//!
//! | Locality Tier | Compute Backend | Rationale |
//! |:-------------:|:---------------:|:----------|
//! | L0 -- L2 | CPU SIMD | Same process/host: Rust `std::simd` or Mojo CPU path |
//! | L3 -- L5 | GPU | Same machine or loopback: Mojo GPU kernel dispatch |
//! | L6 -- L8 | Remote | LAN/WAN/OOB: RPC + pre-serialized frame buffers |
//!
//! Mojo's compile-time targeting means a single kernel can be compiled for
//! both CPU and GPU backends, and the dispatch layer picks the right one at
//! runtime based on `LocalityTier`.
//!
//! # Modules
//!
//! - [`bridge`]: FFI types representing what a Mojo backend would expose.
//! - [`dispatch`]: Maps locality tiers to compute backends.
//! - [`kernel`]: Example kernel interface with CPU fallback.
//! - [`plan`]: GPU dispatch plans with per-tier latency and memory estimates.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod bridge;
pub mod dispatch;
pub mod kernel;
pub mod plan;

pub use bridge::{MojoFrameBuffer, MojoKernel, MojoSurfaceConfig, MojoSurfaceError};
pub use dispatch::DispatchPlan;
pub use kernel::FrameKernel;
pub use plan::GpuDispatchPlan;

/// The compute backend selected by the dispatch layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ComputeBackend {
    /// CPU SIMD (Rust std::simd or Mojo CPU path).
    CpuSimd,
    /// GPU kernel (CUDA / ROCm / Metal via Mojo MLIR).
    Gpu,
    /// Remote RPC + pre-serialized frame buffers.
    RemoteRpc,
}
