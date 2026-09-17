//! Application-side manifest for the checker.
//!
//! A `CheckerManifest` represents what the application *requires* from a host.
//! It is the "wish" side of the check — the `CapabilityDescriptor` is the
//! "truth" side. The checker compares the two and emits a `Decision`.

use serde::{Deserialize, Serialize};

/// What the application needs from a host to run.
///
/// This is a simplified, checker-friendly view of requirements. The route
/// compiler and deployment runtime may have richer types; the checker only
/// cares about the fields it actually checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckerManifest {
    /// Minimum host memory in bytes.
    pub memory_bytes: u64,
    /// Minimum number of CPU cores.
    pub cpu_cores: u32,
    /// Minimum host storage in bytes.
    pub storage_bytes: u64,
    /// Allowed OS families (e.g. "linux", "macos", "windows").
    pub os_families: Vec<String>,
    /// Allowed CPU architectures (e.g. "x86_64", "aarch64").
    pub arches: Vec<String>,
    /// Whether the application requires an audio backend.
    pub audio: bool,
    /// Network peers the application must be able to reach.
    pub network_peers: Vec<String>,
    /// Whether the application can run headless (false = needs a display).
    pub headless: bool,
    /// Whether the application requires a real-time scheduling island.
    pub realtime_island: bool,
}
