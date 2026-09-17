//! Routing types: route plans, topology, and scoring.

use std::collections::BTreeMap;

use anyhow::bail;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::edges::Edge;
use super::ids::{EdgeId, IntentId, NodeId, RoutePlanId, TopologyEpoch};
use super::nodes::Node;


// ---------------------------------------------------------------------------
// Route plan
// ---------------------------------------------------------------------------

/// A single hop in a route plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStep {
    /// The node this step executes on.
    pub node: NodeId,
    /// The capability descriptor to use at this hop (descriptor_id reference).
    pub capability_id: Option<String>,
    /// Edge used to reach this node (if not the first hop).
    pub via_edge: Option<EdgeId>,
    /// What this hop does (e.g. "execute", "route", "sink").
    pub action: String,
}

/// A compiled route plan: the output of the topology compiler.
///
/// A route plan is the concrete answer to "how do I get from here to there,
/// satisfying this intent?" It is pinned to a topology epoch and is only
/// valid until the topology changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePlan {
    /// Unique route plan identifier.
    pub id: RoutePlanId,
    /// Intent this plan satisfies.
    pub intent_id: IntentId,
    /// Topology epoch at which this plan was compiled.
    pub topology_epoch: TopologyEpoch,
    /// Ordered list of hops. May be a single hop (direct) or multi-hop.
    pub steps: Vec<RouteStep>,
    /// Estimated end-to-end latency in microseconds.
    pub estimated_latency_us: Option<f64>,
    /// Score breakdown for this plan (for debugging/audit).
    pub score: Option<ScoreBreakdown>,
    /// When this plan was compiled.
    pub compiled_at: DateTime<Utc>,
    /// When this plan expires (computed from intent expiry + slack).
    pub expires_at: DateTime<Utc>,
    /// Tags inherited from the intent.
    pub tags: Vec<String>,
}

impl RoutePlan {
    /// Validate the plan: all nodes exist, all edges exist, epoch matches.
    pub fn validate(&self, topology: &Topology) -> anyhow::Result<()> {
        if self.topology_epoch != topology.epoch {
            bail!(
                "RoutePlan epoch {} does not match topology epoch {}",
                self.topology_epoch,
                topology.epoch
            );
        }
        if self.steps.is_empty() {
            bail!("RoutePlan has no steps");
        }
        for step in &self.steps {
            if !topology.nodes.contains_key(&step.node) {
                bail!("RoutePlan references unknown node: {}", step.node);
            }
            if let Some(ref edge_id) = step.via_edge {
                if !topology.edges.contains_key(edge_id) {
                    bail!("RoutePlan references unknown edge: {}", edge_id);
                }
            }
        }
        Ok(())
    }

    /// Whether this plan is directly executable (single hop, same process or same machine).
    pub fn is_local(&self) -> bool {
        if self.steps.len() != 1 {
            return false;
        }
        let step = &self.steps[0];
        if step.via_edge.is_some() {
            // Edge-based plan: locality depends on edge tier (not known here).
            // Without topology access, assume non-local.
            return false;
        }
        // Single-hop, no via_edge — locally executable.
        self.steps.len() == 1
    }
}

// ---------------------------------------------------------------------------
// Topology
// ---------------------------------------------------------------------------

/// The full topology graph: nodes, edges, and epoch.
///
/// This is the root data structure that the compiler operates on.
/// It is append-only with respect to epoch changes; old snapshots are preserved
/// for audit purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topology {
    /// Monotonically increasing epoch counter.
    pub epoch: TopologyEpoch,
    /// All nodes in the topology, keyed by NodeId.
    pub nodes: BTreeMap<NodeId, Node>,
    /// All edges in the topology, keyed by EdgeId.
    pub edges: BTreeMap<EdgeId, Edge>,
    /// Topology-wide metadata.
    pub meta: TopologyMeta,
    /// History: snapshots of prior epochs (optional, kept for audit).
    #[serde(skip)]
    history: Vec<Topology>,
}

impl Default for Topology {
    fn default() -> Self {
        Self::new()
    }
}

impl Topology {
    pub fn new() -> Self {
        Self {
            epoch: TopologyEpoch::default(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            meta: TopologyMeta::default(),
            history: Vec::new(),
        }
    }

    /// Add a node to the topology. Idempotent: updates existing nodes.
    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id.clone(), node);
        self.bump_epoch();
    }

    /// Add an edge to the topology.
    /// Returns an error if either endpoint is missing.
    pub fn add_edge(&mut self, edge: Edge) -> anyhow::Result<()> {
        if !self.nodes.contains_key(&edge.from) {
            bail!("Edge references unknown source node: {}", edge.from);
        }
        if !self.nodes.contains_key(&edge.to) {
            bail!("Edge references unknown destination node: {}", edge.to);
        }
        self.edges.insert(edge.id.clone(), edge);
        self.bump_epoch();
        Ok(())
    }

    /// Get a node by ID.
    pub fn node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Get an edge by ID.
    pub fn edge(&self, id: &EdgeId) -> Option<&Edge> {
        self.edges.get(id)
    }

    /// All edges incident to a given node.
    pub fn edges_for(&self, node: &NodeId) -> Vec<&Edge> {
        self.edges
            .values()
            .filter(|e| e.from == *node || e.to == *node)
            .collect()
    }

    /// Bump the topology epoch, archiving the current state.
    fn bump_epoch(&mut self) {
        // Archive current topology before mutating
        let snapshot = Topology {
            epoch: self.epoch,
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            meta: self.meta.clone(),
            history: Vec::new(),
        };
        self.history.push(snapshot);
        let _ = self.epoch.bump();
    }

    /// Number of nodes in the topology.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges in the topology.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// Metadata about the topology as a whole.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopologyMeta {
    /// Human-readable topology name (e.g. "home-lab-v3").
    pub name: String,
    /// When this topology was first created.
    pub created_at: Option<DateTime<Utc>>,
    /// Who created this topology.
    pub created_by: Option<String>,
    /// Arbitrary key-value metadata.
    pub annotations: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Score (re-exported from score.rs)
// ---------------------------------------------------------------------------

/// The final composite score for a route plan.
///
/// Higher is better. Used to rank competing candidates and for audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub locality_score: f64,
    pub latency_score: f64,
    pub capability_score: f64,
    pub trust_score: f64,
    pub composite: f64,
}

impl ScoreBreakdown {
    pub fn new(
        locality_score: f64,
        latency_score: f64,
        capability_score: f64,
        trust_score: f64,
    ) -> Self {
        let composite =
            locality_score * 0.35 + latency_score * 0.30 + capability_score * 0.25 + trust_score * 0.10;
        Self {
            locality_score,
            latency_score,
            capability_score,
            trust_score,
            composite,
        }
    }
}

/// Individual route scoring result (used by the score module).
#[derive(Debug, Clone)]
pub struct Score {
    pub breakdown: ScoreBreakdown,
    pub rank: usize,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::{EdgeId, IntentId, NodeId, RoutePlanId, TopologyEpoch};
    use fabric_capability::locality::LocalityTier;

    #[test]
    fn test_route_plan_validate() {
        let mut topology = Topology::new();
        topology.add_node(Node::new(NodeId::new("a"), LocalityTier::L1));
        topology.add_node(Node::new(NodeId::new("b"), LocalityTier::L2));
        let _ = topology.add_edge(Edge::new(
            EdgeId::new("a-b"),
            NodeId::new("a"),
            NodeId::new("b"),
            LocalityTier::L1,
        ));

        let plan = RoutePlan {
            id: RoutePlanId::new(),
            intent_id: IntentId::new(),
            topology_epoch: topology.epoch,
            steps: vec![RouteStep {
                node: NodeId::new("a"),
                capability_id: None,
                via_edge: None,
                action: "execute".to_string(),
            }],
            estimated_latency_us: Some(10.0),
            score: None,
            compiled_at: Utc::now(),
            expires_at: Utc::now(),
            tags: vec![],
        };

        assert!(plan.validate(&topology).is_ok());
    }

    #[test]
    fn test_route_plan_validate_bad_epoch() {
        let mut topology = Topology::new();
        topology.add_node(Node::new(NodeId::new("a"), LocalityTier::L1));

        let bad_plan = RoutePlan {
            id: RoutePlanId::new(),
            intent_id: IntentId::new(),
            topology_epoch: TopologyEpoch(999),
            steps: vec![RouteStep {
                node: NodeId::new("a"),
                capability_id: None,
                via_edge: None,
                action: "execute".to_string(),
            }],
            estimated_latency_us: None,
            score: None,
            compiled_at: Utc::now(),
            expires_at: Utc::now(),
            tags: vec![],
        };

        assert!(bad_plan.validate(&topology).is_err());
    }

    #[test]
    fn test_topology_add_node_bumps_epoch() {
        let mut topo = Topology::new();
        let e0 = topo.epoch;
        topo.add_node(Node::new(NodeId::new("n1"), LocalityTier::L2));
        assert!(topo.epoch.0 > e0.0);
        let e1 = topo.epoch;
        topo.add_node(Node::new(NodeId::new("n2"), LocalityTier::L1));
        assert!(topo.epoch.0 > e1.0);
    }

    #[test]
    fn test_topology_add_edge_unknown_node() {
        let mut topo = Topology::new();
        topo.add_node(Node::new(NodeId::new("a"), LocalityTier::L1));
        let result = topo.add_edge(Edge::new(
            EdgeId::new("a-b"),
            NodeId::new("a"),
            NodeId::new("nonexistent"),
            LocalityTier::L2,
        ));
        assert!(result.is_err());
    }
}
