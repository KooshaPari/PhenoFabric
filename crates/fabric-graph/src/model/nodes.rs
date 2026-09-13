//! Node types for the topology graph.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ids::NodeId;
use super::types::{CapabilityRef, TrustLevel};

/// A node in the topology graph.
///
/// Each node represents a physical machine, VM, accelerator, or other execution
/// context. It carries zero or more capability descriptors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Node {
    /// Unique node identifier (hostname, IP, or stable UUID).
    pub id: NodeId,
    /// Human-readable label (optional).
    pub label: Option<String>,
    /// Primary locality tier of this node. Used for copy-path reasoning.
    #[cfg_attr(feature = "schemars", schemars(with = "u8"))]
    pub locality_tier: fabric_capability::locality::LocalityTier,
    /// All capability descriptors advertised by this node.
    /// A node with an empty vec has not been probed (or is a pure router).
    pub capabilities: Vec<CapabilityRef>,
    /// When this node was last seen in the topology.
    pub last_seen: DateTime<Utc>,
    /// Extra node-level tags (e.g. "gpu-pool", "high-memory", "rt-island").
    pub tags: Vec<String>,
}

impl Node {
    pub fn new(id: NodeId, locality_tier: fabric_capability::locality::LocalityTier) -> Self {
        Self {
            id,
            label: None,
            locality_tier,
            capabilities: Vec::new(),
            last_seen: Utc::now(),
            tags: Vec::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_capability(mut self, cap: CapabilityRef) -> Self {
        self.capabilities.push(cap);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Whether this node meets the minimum trust requirement.
    pub fn meets_trust(&self, minimum: TrustLevel) -> bool {
        for cap in &self.capabilities {
            if !cap.trust.satisfies(minimum) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::NodeId;
    use crate::model::types::{CapabilityRef, TrustLevel};
    use fabric_capability::locality::LocalityTier;

    #[test]
    fn test_node_with_capabilities() {
        let cap = CapabilityRef::new("sha256:abc123".to_string()).with_trust(TrustLevel::Attested);
        let node = Node::new(NodeId::new("gpu-0"), LocalityTier::L2)
            .with_capability(cap)
            .with_tag("rt-island");

        assert_eq!(node.capabilities.len(), 1);
        assert_eq!(node.tags, vec!["rt-island"]);
        assert!(node.meets_trust(TrustLevel::Untrusted));
        assert!(node.meets_trust(TrustLevel::Bootstrap));
        assert!(node.meets_trust(TrustLevel::Attested));
        assert!(!node.meets_trust(TrustLevel::Audited));
    }
}
