//! Intent types for the topology compiler.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ids::{IntentId, NodeId};
use super::nodes::Node;
use super::types::TrustLevel;

/// The desired execution context for a placed object.
///
/// An intent is the caller-side description of where something should run.
/// The compiler translates an intent into a route plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Unique intent identifier.
    pub id: IntentId,
    /// Human-readable name for this intent (e.g. "ML training on GPU-0").
    pub name: String,
    /// Requirements on the execution context.
    pub requirements: IntentRequirements,
    /// Optional: preferred destination node (soft hint, compiler may override).
    pub preferred_node: Option<NodeId>,
    /// Minimum trust level required for all capabilities.
    pub min_trust: TrustLevel,
    /// When this intent expires (None = no expiry).
    pub expires_at: Option<DateTime<Utc>>,
    /// Tags that the caller wants attached to the resulting route plan.
    pub tags: Vec<String>,
}

/// Core requirements for an intent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntentRequirements {
    /// Required CPU architecture(s). Empty = any.
    pub cpu_arch: Vec<String>,
    /// Minimum RAM in bytes.
    pub min_ram_bytes: Option<u64>,
    /// Required GPU count (None = no GPU needed).
    pub min_gpu_count: Option<u32>,
    /// Required GPU compute capability (e.g. "8.0" for CUDA 8.0).
    pub min_gpu_compute: Option<String>,
    /// Required locality tier (or better). None = any.
    pub max_locality_tier: Option<f64>,
    /// Required OS platform(s). Empty = any.
    pub platforms: Vec<String>,
    /// Required tags (all must be present on the destination node).
    pub required_tags: Vec<String>,
    /// Maximum acceptable latency in microseconds (end-to-end).
    pub max_latency_us: Option<f64>,
    /// Minimum required bandwidth in bytes per second.
    pub min_bandwidth_bps: Option<u64>,
    /// Whether this intent requires a real-time island.
    pub requires_rt_island: bool,
    /// Whether this intent requires GPU direct (NVLink/P2P).
    pub requires_gpu_direct: bool,
}

impl IntentRequirements {
    /// Check whether a node satisfies these requirements.
    pub fn matches_node(&self, node: &Node) -> bool {
        // Check required tags
        for tag in &self.required_tags {
            if !node.tags.contains(tag) {
                return false;
            }
        }
        // Check max locality tier
        if let Some(max) = self.max_locality_tier {
            if (node.locality_tier.as_f64()) > max {
                return false;
            }
        }
        // Check trust level (if node has capabilities)
        // CPU arch, RAM, GPU are checked against descriptors separately
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::NodeId;
    use fabric_capability::locality::LocalityTier;

    #[test]
    fn test_intent_requirements_matches_node() {
        let node = Node::new(NodeId::new("gpu-0"), LocalityTier::L3).with_tag("rt-island");

        let reqs = IntentRequirements {
            required_tags: vec!["rt-island".to_string()],
            max_locality_tier: Some(3.5),
            ..Default::default()
        };
        assert!(reqs.matches_node(&node));

        let reqs_fail = IntentRequirements {
            required_tags: vec!["fpga".to_string()],
            max_locality_tier: Some(3.5),
            ..Default::default()
        };
        assert!(!reqs_fail.matches_node(&node));
    }
}
