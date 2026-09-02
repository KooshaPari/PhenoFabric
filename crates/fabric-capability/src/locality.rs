//! Locality tiers for Fabric topology placement.
//!
//! Fabric categorizes resource-to-resource relationships by their communication
//! cost. The tiers are ordered from cheapest/fastest (L0) to most expensive (L8).

/// A locality tier representing the communication cost between two endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[derive(schemars::JsonSchema)]
pub enum LocalityTier {
    /// Same NUMA node, same process.
    L0SameProcess,
    /// Same NUMA node, different process on same host.
    L1SameNuma,
    /// Different NUMA node, same host, shared memory possible.
    L2CrossNumaShm,
    /// Different NUMA node, same host, PCIe peer-to-peer DMA.
    L3PcieP2P,
    /// Same host, RDMA (RoCE / iWARP / InfiniBand).
    L4Rdma,
    /// Loopback (127.0.0.1 / ::1).
    L5Loopback,
    /// Same subnet / LAN.
    L6Lan,
    /// Different subnet / WAN.
    L7Wan,
    /// Out-of-band: IPMI, KVM-over-IP, Wake-on-LAN.
    L8Oob,
}

impl LocalityTier {
    /// Returns the canonical short code used in descriptor identifiers.
    ///
    /// E.g. `LocalityTier::L0SameProcess` → `"L0"`.
    pub fn short_code(&self) -> &'static str {
        match self {
            LocalityTier::L0SameProcess => "L0",
            LocalityTier::L1SameNuma => "L1",
            LocalityTier::L2CrossNumaShm => "L2",
            LocalityTier::L3PcieP2P => "L3",
            LocalityTier::L4Rdma => "L4",
            LocalityTier::L5Loopback => "L5",
            LocalityTier::L6Lan => "L6",
            LocalityTier::L7Wan => "L7",
            LocalityTier::L8Oob => "L8",
        }
    }

    /// Returns the full human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            LocalityTier::L0SameProcess => "Same NUMA node, same process",
            LocalityTier::L1SameNuma => "Same NUMA node, different process",
            LocalityTier::L2CrossNumaShm => "Cross-NUMA, shared memory possible",
            LocalityTier::L3PcieP2P => "Cross-NUMA, PCIe peer-to-peer DMA",
            LocalityTier::L4Rdma => "RDMA (RoCE / iWARP / InfiniBand)",
            LocalityTier::L5Loopback => "Loopback (127.0.0.1 / ::1)",
            LocalityTier::L6Lan => "Same subnet / LAN",
            LocalityTier::L7Wan => "Different subnet / WAN",
            LocalityTier::L8Oob => "Out-of-band (IPMI, KVM-over-IP, WoL)",
        }
    }
}

impl std::fmt::Display for LocalityTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.short_code(), self.description())
    }
}

/// The 8 copy-path tiers used in link metrics.
///
/// These are a subset of locality tiers that specifically describe the
/// available data-transfer paths between two nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[derive(schemars::JsonSchema)]
pub enum CopyPath {
    /// Same NUMA node, shared memory.
    SharedMemory,
    /// Cross-NUMA, same host.
    CrossNumaShm,
    /// PCIe peer-to-peer DMA.
    PcieP2P,
    /// RDMA (RoCE / iWARP / IB).
    Rdma,
    /// Loopback.
    Loopback,
    /// LAN TCP.
    LanTcp,
    /// WAN TCP.
    WanTcp,
    /// Out-of-band.
    Oob,
}
