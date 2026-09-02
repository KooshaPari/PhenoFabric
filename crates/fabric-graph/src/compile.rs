//! Route compiler: translate intents + topology into route plans.
//!
//! This is the **core compiler** for Fabric's graph-native placement system.
//! It takes an intent and a topology and produces zero or more route plans.
//!
//! ## Algorithm
//!
//! 1. Run [`negotiate`](crate::negotiate) to get ranked candidates
//! 2. Build a route plan for each candidate (single-hop or multi-hop)
//! 3. Validate each plan against the topology
//! 4. Return the best plan (or all candidates if `all` mode)

use crate::model::{Edge, EdgeId, Intent, IntentId, Node, NodeId, RoutePlan, RoutePlanId, RouteStep, Score, Topology, TopologyEpoch};
use crate::negotiation::{negotiate, NegotiationResult};
use chrono::{Duration, Utc};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("no candidate satisfied intent `{intent}`")]
    NoCandidate { intent: String },

    #[error("topology epoch mismatch: plan={plan_epoch}, current={current_epoch}")]
    EpochMismatch { plan_epoch: TopologyEpoch, current_epoch: TopologyEpoch },

    #[error("topology is empty (no nodes)")]
    EmptyTopology,

    #[error("intent expired at {0}")]
    IntentExpired(String),

    #[error("capability descriptor `{0}` not found in node `{1}`")]
    CapabilityNotFound(String, String),
}

/// Result of a compile operation: either a single plan or all candidates.
#[derive(Debug, Clone)]
pub enum CompileResult {
    /// The best single plan.
    Plan(RoutePlan),
    /// All ranked candidates (no plan built yet).
    Candidates(NegotiationResult),
}

/// Compile an intent against a topology into a route plan.
pub fn compile(topology: &Topology, intent: &Intent) -> Result<RoutePlan, CompileError> {
    if topology.nodes.is_empty() {
        return Err(CompileError::EmptyTopology);
    }

    // Check if intent is expired
    if let Some(expires) = intent.expires_at {
        if Utc::now() > expires {
            return Err(CompileError::IntentExpired(expires.to_rfc3339()));
        }
    }

    let result = negotiate(topology, intent);

    let best = result.candidates.first().ok_or_else(|| {
        CompileError::NoCandidate {
            intent: intent.name.clone(),
        }
    })?;

    let steps = build_steps(topology, &best.node, intent)?;

    let estimated_latency = estimate_latency(topology, &steps);
    let compiled_at = Utc::now();
    let expires_at = intent
        .expires_at
        .unwrap_or(compiled_at + Duration::hours(1));

    let plan = RoutePlan {
        id: RoutePlanId::new(),
        intent_id: intent.id.clone(),
        topology_epoch: topology.epoch,
        steps,
        estimated_latency_us: estimated_latency,
        score: Some(best.score.clone()),
        compiled_at,
        expires_at,
        tags: intent.tags.clone(),
    };

    // Validate before returning
    plan.validate(topology).map_err(|_| {
        // Validation failure means topology is inconsistent — treat as epoch mismatch
        CompileError::EpochMismatch {
            plan_epoch: plan.topology_epoch,
            current_epoch: topology.epoch,
        }
    })?;

    Ok(plan)
}

/// Compile all candidates without building plans.
pub fn compile_all(topology: &Topology, intent: &Intent) -> Result<NegotiationResult, CompileError> {
    if topology.nodes.is_empty() {
        return Err(CompileError::EmptyTopology);
    }
    Ok(negotiate(topology, intent))
}

fn build_steps(topology: &Topology, dest: &NodeId, intent: &Intent) -> Result<Vec<RouteStep>, CompileError> {
    // Find the best edge to reach dest (if any)
    let best_edge = topology
        .edges_for(dest)
        .into_iter()
        .filter(|e| e.up)
        .min_by(|a, b| {
            let lat_a = a.metrics.as_ref().and_then(|m| m.latency_us).unwrap_or(f64::INFINITY);
            let lat_b = b.metrics.as_ref().and_then(|m| m.latency_us).unwrap_or(f64::INFINITY);
            lat_a.partial_cmp(&lat_b).unwrap_or(std::cmp::Ordering::Equal)
        });

    let (node, via_edge, action) = if let Some(edge) = best_edge {
        let upstream = if edge.from == *dest { &edge.to } else { &edge.from };
        let upstream_node = topology.node(upstream).expect("edge references valid node");
        if intent.preferred_node.as_ref().map_or(false, |n| n == upstream) {
            // Source is already the preferred node — direct execution
            (upstream.clone(), None, "source-execute".to_string())
        } else {
            (dest.clone(), Some(edge.id.clone()), "route".to_string())
        }
    } else {
        // No edge — assume this is a source node (local execution)
        (dest.clone(), None, "local-execute".to_string())
    };

    // Capability selection: pick the best capability for this node + intent
    let capability_id = select_capability(topology, &node, intent)?;

    Ok(vec![RouteStep {
        node,
        capability_id,
        via_edge,
        action,
    }])
}

fn select_capability(
    topology: &Topology,
    node: &NodeId,
    intent: &Intent,
) -> Result<Option<String>, CompileError> {
    let node_data = topology.node(node).ok_or_else(|| {
        CompileError::CapabilityNotFound("unknown".to_string(), node.to_string())
    })?;

    if node_data.capabilities.is_empty() {
        return Ok(None);
    }

    // Simple strategy: pick the first capability that meets minimum trust
    for cap in &node_data.capabilities {
        if cap.trust.satisfies(intent.min_trust) {
            return Ok(Some(cap.descriptor_id.clone()));
        }
    }

    // None meet trust — return the first one and let the runtime handle it
    Ok(Some(node_data.capabilities[0].descriptor_id.clone()))
}

fn estimate_latency(topology: &Topology, steps: &[RouteStep]) -> Option<f64> {
    let mut total = 0.0;
    for step in steps {
        if let Some(ref edge_id) = step.via_edge {
            if let Some(edge) = topology.edge(edge_id) {
                total += edge.metrics.as_ref().and_then(|m| m.latency_us).unwrap_or(0.0);
            }
        } else {
            // Local hop — negligible latency
            total += 0.0;
        }
    }
    if total == 0.0 {
        None
    } else {
        Some(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CapabilityRef, EdgeId, IntentRequirements, IntentId, NodeId, TopologyEpoch};
    use fabric_capability::locality::LocalityTier;

    fn make_topology() -> Topology {
        let mut topo = Topology::new();
        topo.add_node(
            Node::new(NodeId::new("gpu-0"), LocalityTier::L1)
                .with_tag("rt-island")
                .with_capability(
                    CapabilityRef::new("sha256:gpu0".to_string())
                        .with_trust(crate::model::TrustLevel::Attested),
                ),
        );
        topo.add_node(
            Node::new(NodeId::new("cpu-0"), LocalityTier::L2)
                .with_capability(CapabilityRef::new("sha256:cpu0".to_string())),
        );
        topo.add_edge(Edge::new(
            EdgeId::new("gpu-0-cpu-0"),
            NodeId::new("gpu-0"),
            NodeId::new("cpu-0"),
            LocalityTier::L1,
        ))
        .unwrap();
        topo
    }

    fn make_intent(name: &str) -> Intent {
        Intent {
            id: IntentId::new(),
            name: name.to_string(),
            requirements: IntentRequirements {
                required_tags: vec!["rt-island".to_string()],
                max_locality_tier: Some(5.0),
                ..Default::default()
            },
            preferred_node: None,
            min_trust: crate::model::TrustLevel::Untrusted,
            expires_at: None,
            tags: vec![],
        }
    }

    #[test]
    fn test_compile_single_candidate() {
        let topo = make_topology();
        let intent = make_intent("GPU task");
        let plan = compile(&topo, &intent).expect("should compile");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].node.0, "gpu-0");
        assert_eq!(plan.topology_epoch, topo.epoch);
        assert!(plan.validate(&topo).is_ok());
    }

    #[test]
    fn test_compile_empty_topology() {
        let topo = Topology::new();
        let intent = make_intent("empty");
        let err = compile(&topo, &intent).expect_err("should error");
        assert!(matches!(err, CompileError::EmptyTopology));
    }

    #[test]
    fn test_compile_all() {
        let topo = make_topology();
        let intent = make_intent("all candidates");
        let result = compile_all(&topo, &intent).expect("should succeed");
        assert!(!result.candidates.is_empty());
        assert_eq!(result.candidates[0].node.0, "gpu-0");
    }

    #[test]
    fn test_compile_with_preferred_node() {
        let mut topo = make_topology();
        topo.add_node(
            Node::new(NodeId::new("preferred"), LocalityTier::L1)
                .with_tag("rt-island")
                .with_capability(CapabilityRef::new("sha256:pref".to_string())),
        );

        let mut intent = make_intent("preferred");
        intent.preferred_node = Some(NodeId::new("preferred"));

        let plan = compile(&topo, &intent).expect("should compile");
        // Should prefer the preferred node
        assert!(plan.steps.iter().any(|s| s.node.to_string().contains("preferred")));
    }

    #[test]
    fn test_compile_no_matching_candidate() {
        // Build a topology with NO fpga nodes
        let mut topo = Topology::new();
        topo.add_node(
            Node::new(NodeId::new("cpu-0"), LocalityTier::L2)
                .with_tag("cpu")
                .with_capability(CapabilityRef::new("sha256:cpu0".to_string())),
        );

        let mut intent = make_intent("needs-fpga");
        intent.requirements.required_tags = vec!["fpga".to_string()];

        let err = compile(&topo, &intent).expect_err("should fail");
        assert!(matches!(err, CompileError::NoCandidate { .. }));
    }

    #[test]
    fn test_compile_plan_validates_epoch() {
        let topo = make_topology();
        let intent = make_intent("epoch test");
        let plan = compile(&topo, &intent).expect("should compile");

        // Mutate topology (bump epoch)
        let mut topo2 = topo.clone();
        topo2.add_node(Node::new(NodeId::new("new-node"), LocalityTier::L3));

        // Plan should fail validation against new topology
        assert!(plan.validate(&topo2).is_err());
    }
}
