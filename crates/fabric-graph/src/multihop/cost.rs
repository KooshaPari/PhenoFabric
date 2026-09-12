//! Cost model for multi-hop route plans.

use super::stages::StageCost;
use crate::model::{RoutePlan, Topology};
use serde::{Deserialize, Serialize};

/// Aggregate cost across an entire multi-hop route plan.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouteCost {
    /// Total setup cost across all stages (microseconds).
    pub setup_us: u64,
    /// Total per-frame cost across all stages (microseconds).
    pub per_frame_us: f64,
    /// Peak memory usage across all hops (bytes).
    pub memory_bytes: u64,
    /// End-to-end estimated latency including inter-hop transport (microseconds).
    pub latency_us: f64,
    /// Minimum bandwidth along the route (Mbps).
    pub bandwidth_mbps: f64,
}

impl RouteCost {
    /// Whether this route meets a latency budget.
    pub fn meets_latency_budget(&self, budget_us: f64) -> bool {
        self.latency_us <= budget_us
    }

    /// Whether this route fits within a memory budget.
    pub fn meets_memory_budget(&self, budget_bytes: u64) -> bool {
        self.memory_bytes <= budget_bytes
    }

    /// Composite score (lower is better).
    pub fn composite_score(&self) -> f64 {
        self.latency_us + self.per_frame_us + (self.memory_bytes as f64 / 1024.0 / 1024.0)
    }
}

/// Compute the total cost of a route plan given the topology.
///
/// This uses only the topology edge metrics for inter-hop latency;
/// stage costs are not yet attached to RouteSteps (R3.1 adds stage metadata
/// to RouteStep). For now, we estimate from edge metrics alone.
pub fn compute_route_cost(plan: &RoutePlan, topology: &Topology) -> RouteCost {
    let mut cost = RouteCost::default();

    for (i, step) in plan.steps.iter().enumerate() {
        // Inter-hop latency from edge metrics.
        if let Some(ref edge_id) = step.via_edge {
            if let Some(edge) = topology.edges.get(edge_id) {
                if let Some(ref metrics) = edge.metrics {
                    if let Some(lat) = metrics.latency_us {
                        cost.latency_us += lat;
                    }
                    if let Some(bw) = metrics.bandwidth_bps {
                        let bw_mbps = (bw as f64) / 1_000_000.0;
                        if cost.bandwidth_mbps == 0.0 || bw_mbps < cost.bandwidth_mbps {
                            cost.bandwidth_mbps = bw_mbps;
                        }
                    }
                }
            }
        }

        // If no via_edge, this is the first hop (source).
        // If i > 0 and no via_edge, estimate 0 latency (same machine).
        if i > 0 && step.via_edge.is_none() {
            // Same-machine hop: minimal latency.
        }
    }

    // Estimated total latency = sum of edge latencies.
    // per_frame_us and memory_bytes require stage metadata (R3.1).
    cost.setup_us = 0;
    cost.per_frame_us = 0.0;
    cost.memory_bytes = 0;

    cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Edge, EdgeId, NodeId, RoutePlanId, TopologyEpoch};
    use crate::LocalityTier;
    use chrono::Utc;
    use uuid::Uuid;

    fn test_plan_with_edges() -> (RoutePlan, Topology) {
        let mut topo = Topology::new();
        topo.meta.name = "cost-test".into();

        use crate::model::Node;
        let n1 = Node::new(NodeId("src".into()), LocalityTier::L2CrossNumaShm);
        let n2 = Node::new(NodeId("dst".into()), LocalityTier::L6Lan);
        topo.nodes.insert(n1.id.clone(), n1);
        topo.nodes.insert(n2.id.clone(), n2);

        let edge = Edge::new(
            EdgeId("e1".into()),
            NodeId("src".into()),
            NodeId("dst".into()),
            LocalityTier::L6Lan,
        );
        // Don't insert edge since we're using BTreeMap directly
        topo.edges.insert(edge.id.clone(), edge);

        let plan = RoutePlan {
            id: RoutePlanId(Uuid::now_v7()),
            intent_id: crate::model::IntentId(Uuid::now_v7()),
            topology_epoch: TopologyEpoch(1),
            steps: vec![
                crate::model::RouteStep {
                    node: NodeId("src".into()),
                    capability_id: None,
                    via_edge: None,
                    action: "execute".into(),
                },
                crate::model::RouteStep {
                    node: NodeId("dst".into()),
                    capability_id: None,
                    via_edge: Some(EdgeId("e1".into())),
                    action: "receive".into(),
                },
            ],
            estimated_latency_us: None,
            score: None,
            compiled_at: Utc::now(),
            expires_at: Utc::now(),
            tags: vec![],
        };

        (plan, topo)
    }

    #[test]
    fn route_cost_computation() {
        let (plan, topo) = test_plan_with_edges();
        let cost = compute_route_cost(&plan, &topo);
        // Edge has no metrics, so latency should be 0.
        assert_eq!(cost.latency_us, 0.0);
    }

    #[test]
    fn route_cost_meets_budget() {
        let cost = RouteCost {
            latency_us: 500.0,
            ..Default::default()
        };
        assert!(cost.meets_latency_budget(1000.0));
        assert!(!cost.meets_latency_budget(100.0));
    }

    #[test]
    fn route_cost_composite() {
        let cost = RouteCost {
            latency_us: 100.0,
            per_frame_us: 50.0,
            memory_bytes: 1024 * 1024,
            ..Default::default()
        };
        let score = cost.composite_score();
        // 100 + 50 + 1.0 (1MB in MB) = 151.0
        assert!((score - 151.0).abs() < 0.01);
    }

    #[test]
    fn route_cost_default_is_zero() {
        let cost = RouteCost::default();
        assert_eq!(cost.setup_us, 0);
        assert_eq!(cost.per_frame_us, 0.0);
        assert_eq!(cost.memory_bytes, 0);
        assert_eq!(cost.latency_us, 0.0);
    }
}
