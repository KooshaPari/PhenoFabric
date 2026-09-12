//! Route plan validation for multi-hop routes.

use crate::model::{NodeId, RoutePlan, Topology};
use thiserror::Error;

/// Errors from multi-hop route validation.
#[derive(Debug, Error)]
pub enum RouteValidationError {
    #[error("cycle detected: node {0} appears multiple times")]
    CycleDetected(String),

    #[error("unknown node: {0}")]
    UnknownNode(String),

    #[error("missing edge between {0} and {1}")]
    MissingEdge(String, String),

    #[error("empty route plan (no steps)")]
    EmptyPlan,

    #[error("plan epoch {0} does not match topology epoch {1}")]
    EpochMismatch(u64, u64),
}

/// Validate a multi-hop route plan against a topology.
///
/// Checks:
/// 1. Plan is not empty
/// 2. Epoch matches
/// 3. All nodes exist in topology
/// 4. All edges between consecutive steps exist
/// 5. No cycles (each node appears at most once)
pub fn validate_multihop(
    plan: &RoutePlan,
    topology: &Topology,
) -> Result<(), RouteValidationError> {
    if plan.steps.is_empty() {
        return Err(RouteValidationError::EmptyPlan);
    }

    // Epoch check.
    if plan.topology_epoch != topology.epoch {
        return Err(RouteValidationError::EpochMismatch(plan.topology_epoch.0, topology.epoch.0));
    }

    let mut visited = std::collections::HashSet::new();

    for (i, step) in plan.steps.iter().enumerate() {
        // Check node exists.
        if !topology.nodes.contains_key(&step.node) {
            return Err(RouteValidationError::UnknownNode(step.node.0.clone()));
        }

        // Check for cycles.
        if !visited.insert(step.node.0.clone()) {
            return Err(RouteValidationError::CycleDetected(step.node.0.clone()));
        }

        // Check edge for multi-hop (i > 0).
        if i > 0 {
            if let Some(ref edge_id) = step.via_edge {
                if !topology.edges.contains_key(edge_id) {
                    let prev = &plan.steps[i - 1];
                    return Err(RouteValidationError::MissingEdge(prev.node.0.clone(), step.node.0.clone()));
                }
            }
            // Note: via_edge being None for i > 0 means same-machine hop
            // (no edge needed).
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Edge, EdgeId, RoutePlanId, TopologyEpoch, TopologyMeta};
    use crate::LocalityTier;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_topo() -> Topology {
        let mut topo = Topology::new();
        topo.meta = TopologyMeta {
            name: "test".into(),
            ..Default::default()
        };
        topo.epoch = TopologyEpoch(1);

        use crate::model::Node;
        topo.nodes.insert(
            NodeId("a".into()),
            Node::new(NodeId("a".into()), LocalityTier::L2CrossNumaShm),
        );
        topo.nodes.insert(
            NodeId("b".into()),
            Node::new(NodeId("b".into()), LocalityTier::L6Lan),
        );
        topo.nodes.insert(
            NodeId("c".into()),
            Node::new(NodeId("c".into()), LocalityTier::L6Lan),
        );
        topo.edges.insert(
            EdgeId("e_ab".into()),
            Edge::new(
                EdgeId("e_ab".into()),
                NodeId("a".into()),
                NodeId("b".into()),
                LocalityTier::L6Lan,
            ),
        );
        topo.edges.insert(
            EdgeId("e_bc".into()),
            Edge::new(
                EdgeId("e_bc".into()),
                NodeId("b".into()),
                NodeId("c".into()),
                LocalityTier::L6Lan,
            ),
        );
        topo
    }

    fn make_plan(steps: Vec<crate::model::RouteStep>) -> RoutePlan {
        RoutePlan {
            id: RoutePlanId(Uuid::now_v7()),
            intent_id: crate::model::IntentId(Uuid::now_v7()),
            topology_epoch: TopologyEpoch(1),
            steps,
            estimated_latency_us: None,
            score: None,
            compiled_at: Utc::now(),
            expires_at: Utc::now(),
            tags: vec![],
        }
    }

    #[test]
    fn valid_single_hop() {
        let topo = make_topo();
        let plan = make_plan(vec![crate::model::RouteStep {
            node: NodeId("a".into()),
            capability_id: None,
            via_edge: None,
            action: "execute".into(),
        }]);
        assert!(validate_multihop(&plan, &topo).is_ok());
    }

    #[test]
    fn valid_multi_hop() {
        let topo = make_topo();
        let plan = make_plan(vec![
            crate::model::RouteStep {
                node: NodeId("a".into()),
                capability_id: None,
                via_edge: None,
                action: "execute".into(),
            },
            crate::model::RouteStep {
                node: NodeId("b".into()),
                capability_id: None,
                via_edge: Some(EdgeId("e_ab".into())),
                action: "route".into(),
            },
            crate::model::RouteStep {
                node: NodeId("c".into()),
                capability_id: None,
                via_edge: Some(EdgeId("e_bc".into())),
                action: "receive".into(),
            },
        ]);
        assert!(validate_multihop(&plan, &topo).is_ok());
    }

    #[test]
    fn detects_cycle() {
        let topo = make_topo();
        let plan = make_plan(vec![
            crate::model::RouteStep {
                node: NodeId("a".into()),
                capability_id: None,
                via_edge: None,
                action: "execute".into(),
            },
            crate::model::RouteStep {
                node: NodeId("b".into()),
                capability_id: None,
                via_edge: Some(EdgeId("e_ab".into())),
                action: "route".into(),
            },
            crate::model::RouteStep {
                node: NodeId("a".into()),
                capability_id: None,
                via_edge: Some(EdgeId("e_ab".into())),
                action: "receive".into(),
            },
        ]);
        let err = validate_multihop(&plan, &topo).unwrap_err();
        assert!(matches!(err, RouteValidationError::CycleDetected(_)));
    }

    #[test]
    fn detects_unknown_node() {
        let topo = make_topo();
        let plan = make_plan(vec![crate::model::RouteStep {
            node: NodeId("z".into()),
            capability_id: None,
            via_edge: None,
            action: "execute".into(),
        }]);
        let err = validate_multihop(&plan, &topo).unwrap_err();
        assert!(matches!(err, RouteValidationError::UnknownNode(_)));
    }

    #[test]
    fn detects_empty_plan() {
        let topo = make_topo();
        let plan = make_plan(vec![]);
        let err = validate_multihop(&plan, &topo).unwrap_err();
        assert!(matches!(err, RouteValidationError::EmptyPlan));
    }

    #[test]
    fn detects_epoch_mismatch() {
        let topo = make_topo();
        let mut plan = make_plan(vec![crate::model::RouteStep {
            node: NodeId("a".into()),
            capability_id: None,
            via_edge: None,
            action: "execute".into(),
        }]);
        plan.topology_epoch = TopologyEpoch(99);
        let err = validate_multihop(&plan, &topo).unwrap_err();
        assert!(matches!(err, RouteValidationError::EpochMismatch(..)));
    }
}
