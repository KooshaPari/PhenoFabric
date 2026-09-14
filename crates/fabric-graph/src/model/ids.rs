//! Identifier types for the topology graph.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Monotonically increasing epoch counter for topology versioning.
///
/// Each capability advertisement and topology mutation increments the epoch.
/// Route plans are pinned to an epoch: they are only valid if the current
/// epoch matches the epoch they were compiled against.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
pub struct TopologyEpoch(pub u64);

impl TopologyEpoch {
    /// Advance the epoch by one.
    #[must_use]
    pub fn bump(&mut self) -> TopologyEpoch {
        self.0 += 1;
        *self
    }

    /// Increment and return the new epoch value.
    #[must_use]
    pub fn bump_and_get(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}

impl std::fmt::Display for TopologyEpoch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Identifier types
// ---------------------------------------------------------------------------

/// Unique identifier for a node in the topology graph.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an edge (link) between two nodes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub String);

impl EdgeId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an intent (placement requirement).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IntentId(pub Uuid);

impl IntentId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for IntentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a compiled route plan.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RoutePlanId(pub Uuid);

impl RoutePlanId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for RoutePlanId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topology_epoch_bump() {
        let mut epoch = TopologyEpoch::default();
        assert_eq!(epoch.0, 0);
        epoch.bump();
        assert_eq!(epoch.0, 1);
        assert_eq!(epoch.bump_and_get(), 2);
        assert_eq!(epoch.0, 2);
    }

    #[test]
    fn test_node_id_display() {
        let id = NodeId::new("gpu-0");
        assert_eq!(format!("{}", id), "gpu-0");
    }
}
