//! Edge types for the topology graph.

use serde::{Deserialize, Serialize};

use super::ids::{EdgeId, NodeId};
use super::types::LinkMetrics;

/// An edge (link) between two nodes in the topology graph.
///
/// An edge always has a **locality tier** (L0..L8). The tier reflects the
/// physical separation of the two endpoints. Lower tier = better locality.
/// L0 = same process, L1 = same NUMA node, L2 = same machine, L3 = same LAN,
/// L4 = same campus, L5 = same region, L6 = cross-region, L7 = cross-cloud,
/// L8 = satellite/WAN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    /// Source node.
    pub from: NodeId,
    /// Destination node.
    pub to: NodeId,
    /// Locality tier of this link (required -- PF-FR-002).
    pub locality_tier: fabric_capability::locality::LocalityTier,
    /// Measured link metrics (optional -- populated by probing).
    pub metrics: Option<LinkMetrics>,
    /// Whether this edge is currently usable (admin-up, not quarantined).
    pub up: bool,
}

impl Edge {
    pub fn new(
        id: EdgeId,
        from: NodeId,
        to: NodeId,
        locality_tier: fabric_capability::locality::LocalityTier,
    ) -> Self {
        Self {
            id,
            from,
            to,
            locality_tier,
            metrics: None,
            up: true,
        }
    }

    pub fn with_metrics(mut self, metrics: LinkMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }
}
