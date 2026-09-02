//! Probe trait and POSIX reference implementation.
//!
//! A [`Probe`] discovers the capabilities of a node. The trait is platform-agnostic
//! so that macOS and Windows probes can be added in later releases without changing
//! the descriptor types.

use crate::descriptor::CapabilityDescriptor;
use crate::error::Result;
use crate::locality::LocalityTier;

/// A capability probe for a specific platform.
///
/// Implementors discover hardware and software capabilities of the
/// current node and return a [`CapabilityDescriptor`].
pub trait Probe {
    /// Returns the name of the probe (e.g. `"posix-v1"`, `"macos-v1"`, `"windows-v1"`).
    fn name(&self) -> &str;

    /// Returns the version of the probe.
    fn version(&self) -> &str;

    /// Probes the current node and returns a capability descriptor.
    ///
    /// Implementations should:
    /// - Respect the budget: the small (CPU/NUMA/cache) probe must complete
    ///   in under 100ms; the full probe in under 1s.
    /// - Return [`crate::Error::Unsupported`] if the platform is not supported.
    /// - Be deterministic: same hardware + same kernel state + same probe
    ///   version should produce byte-identical descriptors (modulo timestamps).
    fn probe(&self) -> Result<CapabilityDescriptor>;
}

/// The default probe for the current platform.
///
/// On POSIX systems, returns [`PosixProbe`].
#[cfg(target_family = "unix")]
pub fn default_probe() -> Box<dyn Probe> {
    Box::new(PosixProbe::new())
}

/// On non-POSIX platforms, returns an unsupported stub.
#[cfg(not(target_family = "unix"))]
pub fn default_probe() -> Box<dyn Probe> {
    Box::new(UnsupportedProbe::new())
}

// ---------------------------------------------------------------------------
// POSIX probe
// ---------------------------------------------------------------------------

/// POSIX probe — implements Probe for Linux and macOS.
pub struct PosixProbe {
    version: String,
}

impl PosixProbe {
    /// Creates a new POSIX probe.
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl Default for PosixProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl Probe for PosixProbe {
    fn name(&self) -> &str {
        "posix"
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn probe(&self) -> Result<CapabilityDescriptor> {
        let compute = self::cpu::probe_compute()?;
        let capabilities = crate::descriptor::Capabilities {
            compute: Some(compute),
            ..Default::default()
        };

        let mut descriptor = CapabilityDescriptor {
            node_id: uuid::Uuid::now_v7(),
            epoch: 1,
            schema_version: "1.0.0".to_string(),
            probed_at: chrono::Utc::now(),
            probe_version: self.version.clone(),
            topology_hash: String::new(),
            capabilities,
            signatures: vec![],
        };

        descriptor.topology_hash = descriptor.topology_hash();
        Ok(descriptor)
    }
}

mod cpu {
    use super::*;
    use crate::descriptor::{CacheInfo, ComputeCapabilities};
    use crate::error::Error;

    /// Probes CPU/NUMA/cache/memory from /proc and /sys.
    ///
    /// Falls back to a minimal descriptor on macOS (which doesn't expose
    /// NUMA the same way).
    pub fn probe_compute() -> Result<ComputeCapabilities> {
        #[cfg(target_os = "linux")]
        {
            probe_compute_linux()
        }
        #[cfg(target_os = "macos")]
        {
            probe_compute_macos()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(Error::Unsupported(format!(
                "compute probe not implemented for target_os={}",
                std::env::consts::OS
            )))
        }
    }

    #[cfg(target_os = "linux")]
    fn probe_compute_linux() -> Result<ComputeCapabilities> {
        let cpuinfo = std::fs::read_to_string("/proc/cpuinfo")
            .map_err(|e| Error::Io("/proc/cpuinfo".to_string(), e))?;

        // Parse processor count from /proc/cpuinfo
        let cores_logical = cpuinfo
            .matches("processor\t:")
            .count()
            .try_into()
            .unwrap_or(0u32);

        // Parse model name (all cores share the same model)
        let processor = cpuinfo
            .lines()
            .find_map(|l| l.strip_prefix("model name\t: "))
            .unwrap_or("unknown")
            .to_string();

        // Parse physical cores via "cpu cores" entries
        let cores_physical: u32 = cpuinfo
            .lines()
            .find_map(|l| l.strip_prefix("cpu cores\t: "))
            .and_then(|s| s.parse().ok())
            .unwrap_or(cores_logical);

        // Detect NUMA nodes via /sys/devices/system/node/
        let numa_nodes = std::fs::read_dir("/sys/devices/system/node")
            .map_err(|e| Error::Io("/sys/devices/system/node".to_string(), e))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("node"))
            .count()
            .try_into()
            .unwrap_or(1u32);

        // Cache info: read /sys/devices/system/cpu/cpu0/cache/index*/size
        let cache = read_cache_info();

        // Memory from /proc/meminfo
        let memory_bytes = std::fs::read_to_string("/proc/meminfo")
            .map_err(|e| Error::Io("/proc/meminfo".to_string(), e))?
            .lines()
            .find_map(|l| l.strip_prefix("MemTotal: "))
            .and_then(|s| {
                // e.g. "32768000 kB" → 32_768_000 * 1024
                s.split_whitespace()
                    .next()
                    .and_then(|n| n.parse::<u64>().ok())
                    .map(|kb| kb * 1024)
            })
            .unwrap_or(0);

        Ok(ComputeCapabilities {
            processor,
            cores_physical,
            cores_logical,
            numa_nodes,
            cache,
            memory_bytes,
            memory_bandwidth_mbps: None,
            hyperthread_pairs: vec![],
            tdp_watts: None,
        })
    }

    #[cfg(target_os = "linux")]
    fn read_cache_info() -> Vec<CacheInfo> {
        let mut cache = Vec::new();
        let base = std::path::Path::new("/sys/devices/system/cpu/cpu0/cache");
        if !base.exists() {
            return cache;
        }
        let entries = match std::fs::read_dir(base) {
            Ok(e) => e,
            Err(_) => return cache,
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.starts_with("index") {
                continue;
            }
            let path = entry.path();
            let level = std::fs::read_to_string(path.join("level"))
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            let size_kb = std::fs::read_to_string(path.join("size"))
                .ok()
                .and_then(|s| s.trim().trim_end_matches('K').parse::<u64>().ok())
                .unwrap_or(0);
            let line_size = std::fs::read_to_string(path.join("coherency_line_size"))
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(64);
            let cores_sharing = std::fs::read_to_string(path.join("shared_cpu_list"))
                .ok()
                .and_then(|s| s.trim().matches(',').count() as u32)
                .unwrap_or(0)
                + 1;
            if level > 0 {
                cache.push(CacheInfo {
                    level,
                    size_bytes: size_kb * 1024,
                    line_size_bytes: line_size,
                    cores_sharing,
                    numa_node: None,
                });
            }
        }
        cache
    }

    #[cfg(target_os = "macos")]
    fn probe_compute_macos() -> Result<ComputeCapabilities> {
        // macOS: use sysctl for CPU info. Note: hw.logicalcpu/physicalcpu return
        // numeric values; machdep.cpu.brand_string returns "Apple M2 Pro".
        let cores_logical: u32 = sysctl_value("hw.logicalcpu")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let cores_physical: u32 = sysctl_value("hw.physicalcpu")
            .and_then(|s| s.parse().ok())
            .unwrap_or(cores_logical);
        let processor = sysctl_value("machdep.cpu.brand_string")
            .or_else(|| sysctl_value("hw.machine"))
            .unwrap_or_else(|| "Apple Silicon".to_string());
        let memory_bytes: u64 = sysctl_value("hw.memsize")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Ok(ComputeCapabilities {
            processor,
            cores_physical,
            cores_logical,
            numa_nodes: 1,
            cache: vec![],
            memory_bytes,
            memory_bandwidth_mbps: None,
            hyperthread_pairs: vec![],
            tdp_watts: None,
        })
    }

    #[cfg(target_os = "macos")]
    fn sysctl_value(name: &str) -> Option<String> {
        // Use `sysctl -n <name>` subprocess for portability.
        // Real implementation would use libc sysctl directly.
        let output = std::process::Command::new("sysctl")
            .args(["-n", name])
            .output()
            .ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Unsupported stub
// ---------------------------------------------------------------------------

/// A stub probe that always returns `Error::Unsupported`.
pub struct UnsupportedProbe {
    version: String,
}

impl UnsupportedProbe {
    /// Creates a new unsupported probe.
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl Default for UnsupportedProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl Probe for UnsupportedProbe {
    fn name(&self) -> &str {
        "unsupported"
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn probe(&self) -> Result<CapabilityDescriptor> {
        Err(crate::Error::Unsupported(format!(
            "no probe available for target_os={}",
            std::env::consts::OS
        )))
    }
}

#[allow(dead_code)]
fn _ensure_locality_imported(_: LocalityTier) {}
