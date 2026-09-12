//! Multi-hop route plan compiler.
//!
//! Generates route plans that may span multiple topology hops,
//! selecting appropriate transport stages at each hop.

pub mod stages;
pub mod cost;
pub mod validate;
pub mod fallback;

pub use stages::{builtin_stages, TransportStage};
pub use cost::{compute_route_cost, RouteCost};
pub use validate::{validate_multihop, RouteValidationError};

use crate::model::{EdgeId, Intent, NodeId, RoutePlan, RoutePlanId, RouteStep, Topology, TopologyEpoch};
use crate::LocalityTier;
use fallback::generate_fallbacks;
use chrono::Utc;
use thiserror::Error;

/// Errors from multi-hop compilation.
#[derive(Debug, Error)]
pub enum MultihopError {
    #[error("no path found from {0} to {1}")]
    NoPath(String, String),

    #[error("source and destination are the same node: {0}")]
    SameNode(String),

    #[error(transparent)]
    Validation(#[from] RouteValidationError),

    #[error("topology is empty")]
    EmptyTopology,
}

/// Result of multi-hop compilation.
#[derive(Debug, Clone)]
pub struct MultihopResult {
    /// The primary route plan.
    pub primary: RoutePlan,
    /// Cost breakdown.
    pub cost: RouteCost,
    /// Fallback plans (alternative routes).
    pub fallbacks: Vec<RoutePlan>,
    /// Transport stages selected (per-hop).
    pub stages_per_hop: Vec<Vec<TransportStage>>,
}

/// Compile a multi-hop route plan from source to destination.
///
/// Algorithm:
/// 1. BFS from source to destination through topology edges
/// 2. For each hop, select transport stages based on locality tier gap
/// 3. Build route plan from the path
/// 4. Validate the plan
/// 5. Compute cost
/// 6. Generate fallback plans
pub fn compile_multihop(
    topology: &Topology,
    source: &NodeId,
    destination: &NodeId,
    _intent: &Intent,
    catalog: &[TransportStage],
) -> Result<MultihopResult, MultihopError> {
    if topology.nodes.is_empty() {
        return Err(MultihopError::EmptyTopology);
    }

    if source == destination {
        return Err(MultihopError::SameNode(source.0.clone()));
    }

    // 1. BFS to find shortest path.
    let path = bfs_path(topology, source, destination)
        .ok_or_else(|| MultihopError::NoPath(source.0.clone(), destination.0.clone()))?;

    // 2. For each hop, select stages.
    let mut stages_per_hop: Vec<Vec<TransportStage>> = Vec::new();
    for window in path.windows(2) {
        let from_node = topology.nodes.get(&window[0]);
        let to_node = topology.nodes.get(&window[1]);
        let edge = find_edge_between(topology, &window[0], &window[1]);

        let tier = edge
            .map(|e| e.locality_tier)
            .unwrap_or(LocalityTier::L7Wan);

        let stages = select_stages(tier, catalog);
        stages_per_hop.push(stages);
    }

    // 3. Build route plan.
    let mut steps = Vec::new();
    for (i, node_id) in path.iter().enumerate() {
        let via_edge = if i > 0 {
            find_edge_between(topology, &path[i - 1], node_id).map(|e| e.id.clone())
        } else {
            None
        };

        let action = if i == 0 {
            "execute"
        } else if i == path.len() - 1 {
            "receive"
        } else {
            "route"
        };

        steps.push(RouteStep {
            node: node_id.clone(),
            capability_id: None,
            via_edge,
            action: action.into(),
        });
    }

    let plan = RoutePlan {
        id: RoutePlanId(uuid::Uuid::now_v7()),
        intent_id: crate::model::IntentId(uuid::Uuid::now_v7()),
        topology_epoch: topology.epoch,
        steps,
        estimated_latency_us: None,
        score: None,
        compiled_at: Utc::now(),
        expires_at: Utc::now(),
        tags: vec![],
    };

    // 4. Validate.
    validate_multihop(&plan, topology)?;

    // 5. Compute cost.
    let cost = compute_route_cost(&plan, topology);

    // 6. Generate fallbacks.
    let fallbacks = generate_fallbacks(&plan, topology, 2);

    Ok(MultihopResult {
        primary: plan,
        cost,
        fallbacks,
        stages_per_hop,
    })
}

/// BFS to find shortest path from source to destination.
fn bfs_path(topology: &Topology, source: &NodeId, dest: &NodeId) -> Option<Vec<NodeId>> {
    use std::collections::{HashMap, VecDeque};

    let mut visited: HashMap<String, Option<String>> = HashMap::new();
    let mut queue = VecDeque::new();

    visited.insert(source.0.clone(), None);
    queue.push_back(source.clone());

    while let Some(current) = queue.pop_front() {
        if current == *dest {
            // Reconstruct path.
            let mut path = vec![dest.clone()];
            let mut cur = dest.0.clone();
            while let Some(Some(prev)) = visited.get(&cur) {
                path.push(NodeId(prev.clone()));
                cur = prev.clone();
            }
            path.reverse();
            return Some(path);
        }

        // Find neighbors via edges.
        for edge in topology.edges.values() {
            if edge.from == current && edge.up {
                let next = &edge.to;
                if !visited.contains_key(&next.0) {
                    visited.insert(next.0.clone(), Some(current.0.clone()));
                    queue.push_back(next.clone());
                }
            }
        }
    }

    None
}

/// Find an edge between two nodes.
fn find_edge_between<'a>(
    topology: &'a Topology,
    from: &NodeId,
    to: &NodeId,
) -> Option<&'a crate::model::Edge> {
    topology.edges.values().find(|e| e.from == *from && e.to == *to && e.up)
}

/// Select transport stages for a hop given the locality tier.
fn select_stages(tier: LocalityTier, catalog: &[TransportStage]) -> Vec<TransportStage> {
    let mut selected = Vec::new();

    // Identity stage is always available.
    if let Some(identity) = catalog.iter().find(|s| s.id.0 == "identity") {
        selected.push(identity.clone());
    }

    // Select transport based on tier.
    let transport_id = match tier {
        LocalityTier::L0SameProcess | LocalityTier::L1SameNuma | LocalityTier::L2CrossNumaShm => {
            "unix_socket"
        }
        LocalityTier::L5Loopback => "unix_socket",
        LocalityTier::L6Lan | LocalityTier::L7Wan => "quic_transport",
        _ => "tcp_transport",
    };

    if let Some(transport) = catalog.iter().find(|s| s.id.0 == transport_id) {
        selected.push(transport.clone());
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Edge, Node, TopologyMeta};
    use crate::LocalityTier;

    fn make_topo() -> Topology {
        let mut topo = Topology::new();
        topo.meta = TopologyMeta {
            name: "multihop-test".into(),
            ..Default::default()
        };
        topo.epoch = TopologyEpoch(1);

        for (id, tier) in [
            ("src", LocalityTier::L2CrossNumaShm),
            ("relay", LocalityTier::L6Lan),
            ("dst", LocalityTier::L6Lan),
        ] {
            topo.nodes
                .insert(NodeId(id.into()), Node::new(NodeId(id.into()), tier));
        }

        topo.edges.insert(
            EdgeId("e1".into()),
            Edge::new(
                EdgeId("e1".into()),
                NodeId("src".into()),
                NodeId("relay".into()),
                LocalityTier::L6Lan,
            ),
        );
        topo.edges.insert(
            EdgeId("e2".into()),
            Edge::new(
                EdgeId("e2".into()),
                NodeId("relay".into()),
                NodeId("dst".into()),
                LocalityTier::L6Lan,
            ),
        );
        topo
    }

    fn test_intent() -> Intent {
        crate::model::Intent {
            id: crate::model::IntentId(uuid::Uuid::now_v7()),
            name: "test-intent".into(),
            requirements: crate::model::IntentRequirements::default(),
            preferred_node: None,
            min_trust: crate::model::TrustLevel::default(),
            tags: vec![],
            expires_at: None,
        }
    }

    #[test]
    fn compile_multihop_basic() {
        let topo = make_topo();
        let catalog = builtin_stages();
        let intent = test_intent();

        let result = compile_multihop(
            &topo,
            &NodeId("src".into()),
            &NodeId("dst".into()),
            &intent,
            &catalog,
        )
        .unwrap();

        assert_eq!(result.primary.steps.len(), 3);
        assert_eq!(result.primary.steps[0].node.0, "src");
        assert_eq!(result.primary.steps[1].node.0, "relay");
        assert_eq!(result.primary.steps[2].node.0, "dst");
        assert!(!result.stages_per_hop.is_empty());
    }

    #[test]
    fn compile_multihop_no_path() {
        let topo = make_topo();
        let catalog = builtin_stages();
        let intent = test_intent();

        // src and dst exist but "isolated" doesn't exist
        let result = compile_multihop(
            &topo,
            &NodeId("src".into()),
            &NodeId("isolated".into()),
            &intent,
            &catalog,
        );
        assert!(result.is_err());
    }

    #[test]
    fn compile_multihop_same_node() {
        let topo = make_topo();
        let catalog = builtin_stages();
        let intent = test_intent();

        let result = compile_multihop(
            &topo,
            &NodeId("src".into()),
            &NodeId("src".into()),
            &intent,
            &catalog,
        );
        assert!(matches!(result, Err(MultihopError::SameNode(_))));
    }

    #[test]
    fn compile_multihop_empty_topology() {
        let topo = Topology::new();
        let catalog = builtin_stages();
        let intent = test_intent();

        let result = compile_multihop(
            &topo,
            &NodeId("a".into()),
            &NodeId("b".into()),
            &intent,
            &catalog,
        );
        assert!(matches!(result, Err(MultihopError::EmptyTopology)));
    }

    #[test]
    fn bfs_finds_shortest_path() {
        let topo = make_topo();
        let path = bfs_path(&topo, &NodeId("src".into()), &NodeId("dst".into()));
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].0, "src");
        assert_eq!(path[1].0, "relay");
        assert_eq!(path[2].0, "dst");
    }

    #[test]
    fn select_stages_for_lan() {
        let catalog = builtin_stages();
        let stages = select_stages(LocalityTier::L6Lan, &catalog);
        assert!(!stages.is_empty());
        assert!(stages.iter().any(|s| s.id.0 == "identity"));
        assert!(stages.iter().any(|s| s.id.0 == "quic_transport"));
    }

    #[test]
    fn select_stages_for_shm() {
        let catalog = builtin_stages();
        let stages = select_stages(LocalityTier::L0SameProcess, &catalog);
        assert!(!stages.is_empty());
        assert!(stages.iter().any(|s| s.id.0 == "identity"));
        assert!(stages.iter().any(|s| s.id.0 == "unix_socket"));
    }
}
