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

    /// Numeric tier index as f64 (0.0 .. 8.0). Used in scoring and sorting.
    #[must_use]
    pub fn as_f64(self) -> f64 {
        f64::from(locality_index(self))
    }

    /// Parse a tier from its numeric value (0..=8).
    pub fn from_index(i: u8) -> Option<Self> {
        match i {
            0 => Some(LocalityTier::L0SameProcess),
            1 => Some(LocalityTier::L1SameNuma),
            2 => Some(LocalityTier::L2CrossNumaShm),
            3 => Some(LocalityTier::L3PcieP2P),
            4 => Some(LocalityTier::L4Rdma),
            5 => Some(LocalityTier::L5Loopback),
            6 => Some(LocalityTier::L6Lan),
            7 => Some(LocalityTier::L7Wan),
            8 => Some(LocalityTier::L8Oob),
            _ => None,
        }
    }

    /// Short-form aliases matching the Fabric spec's L0..L8 naming.
    pub const L0: LocalityTier = LocalityTier::L0SameProcess;
    pub const L1: LocalityTier = LocalityTier::L1SameNuma;
    pub const L2: LocalityTier = LocalityTier::L2CrossNumaShm;
    pub const L3: LocalityTier = LocalityTier::L3PcieP2P;
    pub const L4: LocalityTier = LocalityTier::L4Rdma;
    pub const L5: LocalityTier = LocalityTier::L5Loopback;
    pub const L6: LocalityTier = LocalityTier::L6Lan;
    pub const L7: LocalityTier = LocalityTier::L7Wan;
    pub const L8: LocalityTier = LocalityTier::L8Oob;
}

impl std::fmt::Display for LocalityTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.short_code(), self.description())
    }
}

/// Parse a short code like "L0".. "L8" into a [`LocalityTier`].
impl std::str::FromStr for LocalityTier {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "L0" => Ok(LocalityTier::L0SameProcess),
            "L1" => Ok(LocalityTier::L1SameNuma),
            "L2" => Ok(LocalityTier::L2CrossNumaShm),
            "L3" => Ok(LocalityTier::L3PcieP2P),
            "L4" => Ok(LocalityTier::L4Rdma),
            "L5" => Ok(LocalityTier::L5Loopback),
            "L6" => Ok(LocalityTier::L6Lan),
            "L7" => Ok(LocalityTier::L7Wan),
            "L8" => Ok(LocalityTier::L8Oob),
            other => Err(format!("Unknown LocalityTier short code: {other}")),
        }
    }
}

/// Numeric locality index for ranking and scoring (lower = better).
pub fn locality_index(t: LocalityTier) -> u8 {
    match t {
        LocalityTier::L0SameProcess => 0,
        LocalityTier::L1SameNuma => 1,
        LocalityTier::L2CrossNumaShm => 2,
        LocalityTier::L3PcieP2P => 3,
        LocalityTier::L4Rdma => 4,
        LocalityTier::L5Loopback => 5,
        LocalityTier::L6Lan => 6,
        LocalityTier::L7Wan => 7,
        LocalityTier::L8Oob => 8,
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
