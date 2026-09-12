//! Fallback route generation for multi-hop routes.

use crate::model::{EdgeId, NodeId, RoutePlan, RouteStep, Topology};
use crate::LocalityTier;

/// Generate fallback route plans for a primary route.
///
/// Strategy: find alternative paths between consecutive hops using
/// edges with different locality tiers. Returns up to `max_fallbacks`
/// alternative plans.
pub fn generate_fallbacks(
    primary: &RoutePlan,
    topology: &Topology,
    max_fallbacks: usize,
) -> Vec<RoutePlan> {
    if primary.steps.len() <= 1 {
        return vec![];
    }

    let mut fallbacks = Vec::new();

    // Strategy 1: Try alternative edge for each hop.
    for hop_idx in 1..primary.steps.len() {
        if fallbacks.len() >= max_fallbacks {
            break;
        }

        let step = &primary.steps[hop_idx];
        let prev = &primary.steps[hop_idx - 1];

        // Find alternative edges from prev.node to step.node.
        let alt_edges: Vec<&EdgeId> = topology
            .edges
            .values()
            .filter(|e| e.from == prev.node && e.to == step.node && e.up)
            .filter(|e| {
                step.via_edge
                    .as_ref()
                    .map(|vid| *vid != e.id)
                    .unwrap_or(true)
            })
            .map(|e| &e.id)
            .collect();

        for alt_edge in alt_edges {
            if fallbacks.len() >= max_fallbacks {
                break;
            }

            let mut steps = primary.steps.clone();
            steps[hop_idx] = RouteStep {
                node: step.node.clone(),
                capability_id: step.capability_id.clone(),
                via_edge: Some(alt_edge.clone()),
                action: step.action.clone(),
            };

            fallbacks.push(RoutePlan {
                id: crate::model::RoutePlanId(uuid::Uuid::now_v7()),
                intent_id: primary.intent_id.clone(),
                topology_epoch: primary.topology_epoch,
                steps,
                estimated_latency_us: primary.estimated_latency_us,
                score: None,
                compiled_at: primary.compiled_at,
                expires_at: primary.expires_at,
                tags: primary.tags.clone(),
            });
        }
    }

    // Strategy 2: Degraded path — prefer higher-tier (cheaper) edges.
    if fallbacks.len() < max_fallbacks {
        if let Some(degraded) = generate_degraded_path(primary, topology) {
            fallbacks.push(degraded);
        }
    }

    fallbacks
}

/// Generate a degraded-quality fallback (skip transport, direct only).
fn generate_degraded_path(primary: &RoutePlan, topology: &Topology) -> Option<RoutePlan> {
    // Only generate degraded path for multi-hop routes.
    if primary.steps.len() < 2 {
        return None;
    }

    // Degraded: try to find a direct single-hop path from first to last node.
    let src = &primary.steps.first()?.node;
    let dst = &primary.steps.last()?.node;

    // Find direct edge.
    let direct_edge = topology.edges.values().find(|e| {
        e.from == *src && e.to == *dst && e.up
    })?;

    let mut steps = vec![
        RouteStep {
            node: src.clone(),
            capability_id: None,
            via_edge: None,
            action: "execute".into(),
        },
        RouteStep {
            node: dst.clone(),
            capability_id: None,
            via_edge: Some(direct_edge.id.clone()),
            action: "receive".into(),
        },
    ];

    // If src == dst, just one step.
    if src == dst {
        steps.pop();
    }

    Some(RoutePlan {
        id: crate::model::RoutePlanId(uuid::Uuid::now_v7()),
        intent_id: primary.intent_id.clone(),
        topology_epoch: primary.topology_epoch,
        steps,
        estimated_latency_us: None, // Degraded: unknown latency
        score: None,
        compiled_at: primary.compiled_at,
        expires_at: primary.expires_at,
        tags: primary.tags.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Edge, IntentId, RoutePlanId, TopologyEpoch, TopologyMeta};
    use chrono::Utc;
    use uuid::Uuid;

    fn make_topo() -> Topology {
        let mut topo = Topology::new();
        topo.meta = TopologyMeta {
            name: "fallback-test".into(),
            ..Default::default()
        };
        topo.epoch = TopologyEpoch(1);

        use crate::model::Node;
        for id in &["a", "b", "c"] {
            topo.nodes.insert(
                NodeId(id.to_string()),
                Node::new(NodeId(id.to_string()), LocalityTier::L6Lan),
            );
        }
        // Primary path: a→b (L6), b→c (L6)
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
        // Alternative path: a→c direct (L7)
        topo.edges.insert(
            EdgeId("e_ac_direct".into()),
            Edge::new(
                EdgeId("e_ac_direct".into()),
                NodeId("a".into()),
                NodeId("c".into()),
                LocalityTier::L7Wan,
            ),
        );
        topo
    }

    fn make_plan() -> RoutePlan {
        RoutePlan {
            id: RoutePlanId(Uuid::now_v7()),
            intent_id: IntentId(Uuid::now_v7()),
            topology_epoch: TopologyEpoch(1),
            steps: vec![
                RouteStep {
                    node: NodeId("a".into()),
                    capability_id: None,
                    via_edge: None,
                    action: "execute".into(),
                },
                RouteStep {
                    node: NodeId("b".into()),
                    capability_id: None,
                    via_edge: Some(EdgeId("e_ab".into())),
                    action: "route".into(),
                },
                RouteStep {
                    node: NodeId("c".into()),
                    capability_id: None,
                    via_edge: Some(EdgeId("e_bc".into())),
                    action: "receive".into(),
                },
            ],
            estimated_latency_us: Some(1000.0),
            score: None,
            compiled_at: Utc::now(),
            expires_at: Utc::now(),
            tags: vec![],
        }
    }

    #[test]
    fn generates_fallbacks_for_multihop() {
        let topo = make_topo();
        let plan = make_plan();
        let fallbacks = generate_fallbacks(&plan, &topo, 3);
        assert!(!fallbacks.is_empty());
        // Should find at least 1 alternative edge path
    }

    #[test]
    fn no_fallbacks_for_single_hop() {
        let topo = make_topo();
        let plan = RoutePlan {
            id: RoutePlanId(Uuid::now_v7()),
            intent_id: IntentId(Uuid::now_v7()),
            topology_epoch: TopologyEpoch(1),
            steps: vec![RouteStep {
                node: NodeId("a".into()),
                capability_id: None,
                via_edge: None,
                action: "execute".into(),
            }],
            estimated_latency_us: None,
            score: None,
            compiled_at: Utc::now(),
            expires_at: Utc::now(),
            tags: vec![],
        };
        let fallbacks = generate_fallbacks(&plan, &topo, 3);
        assert!(fallbacks.is_empty());
    }

    #[test]
    fn respects_max_fallbacks() {
        let topo = make_topo();
        let plan = make_plan();
        let fallbacks = generate_fallbacks(&plan, &topo, 1);
        assert!(fallbacks.len() <= 1);
    }
}
