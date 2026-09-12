//! Tests for fabric CLI route subcommand.
//!
//! Builds a topology, compiles a route, validates and shows it.

use fabric_graph::builder::{IntentBuilder, TopologyBuilder};
use fabric_graph::compile;
use fabric_capability::locality::LocalityTier;
use fabric_graph::model::{Edge, EdgeId, NodeId, TrustLevel};

fn small_topology() -> fabric_graph::model::Topology {
    TopologyBuilder::new()
        .with_name("route-test")
        .add_with_label("src", "source", LocalityTier::L0SameProcess)
        .add_with_label("dst", "destination", LocalityTier::L6Lan)
        .connect_with_metrics(
            "src",
            "dst",
            LocalityTier::L6Lan,
            fabric_graph::model::LinkMetrics {
                latency_us: Some(500.0),
                bandwidth_bps: Some(1_000_000_000),
                packet_loss: Some(0.001),
                jitter_us: Some(10.0),
            },
        )
        .build()
}

#[test]
fn compile_route_plan() {
    let topo = small_topology();
    let intent = IntentBuilder::new()
        .name("test-route")
        .min_trust(TrustLevel::Untrusted)
        .max_locality(8.0)
        .build();
    let plan = compile(&topo, &intent);
    assert!(plan.is_ok(), "compile should succeed: {:?}", plan.err());
    let plan = plan.unwrap();
    assert!(!plan.steps.is_empty());
    assert_eq!(plan.topology_epoch, topo.epoch);
}

#[test]
fn compile_route_plan_with_tag() {
    let topo = TopologyBuilder::new()
        .with_name("tagged-topo")
        .add_with_tag("gpu-node", LocalityTier::L0SameProcess, "compute")
        .add_simple_node("idle-node", LocalityTier::L6Lan)
        .connect("gpu-node", "idle-node", LocalityTier::L6Lan)
        .build();
    let intent = IntentBuilder::new()
        .name("gpu-workload")
        .min_trust(TrustLevel::Untrusted)
        .require_tag("compute")
        .build();
    let plan = compile(&topo, &intent).unwrap();
    assert!(!plan.steps.is_empty());
    // Should route to gpu-node since it has the "compute" tag
    assert_eq!(plan.steps[0].node, NodeId::new("gpu-node"));
}

#[test]
fn route_plan_serialization_roundtrip() {
    let topo = small_topology();
    let intent = IntentBuilder::new()
        .name("serial-test")
        .build();
    let plan = compile(&topo, &intent).unwrap();
    let json = serde_json::to_string_pretty(&plan).unwrap();
    let loaded: fabric_graph::model::RoutePlan = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.steps.len(), plan.steps.len());
    assert_eq!(loaded.id, plan.id);
}

#[test]
fn compile_with_high_trust_requirement() {
    // Nodes with untrusted capabilities should NOT satisfy audited trust.
    use fabric_graph::model::{CapabilityRef, Node};
    use fabric_graph::builder::TopologyBuilder;

    let topo = TopologyBuilder::new()
        .with_name("trust-test")
        .add(
            Node::new(NodeId::new("src"), LocalityTier::L0SameProcess)
                .with_capability(
                    CapabilityRef::new("cap-a".into())
                        .with_trust(TrustLevel::Untrusted),
                ),
        )
        .add(
            Node::new(NodeId::new("dst"), LocalityTier::L6Lan)
                .with_capability(
                    CapabilityRef::new("cap-b".into())
                        .with_trust(TrustLevel::Untrusted),
                ),
        )
        .connect("src", "dst", LocalityTier::L6Lan)
        .build();
    let intent = IntentBuilder::new()
        .name("high-trust")
        .min_trust(TrustLevel::Audited)
        .build();
    let result = compile(&topo, &intent);
    assert!(result.is_err(), "should fail when trust requirement unmet");
}

#[test]
fn compile_empty_topology_fails() {
    let topo = TopologyBuilder::new().with_name("empty").build();
    let intent = IntentBuilder::new().name("empty-topo").build();
    let result = compile(&topo, &intent);
    assert!(result.is_err(), "should fail on empty topology");
}

#[test]
fn compile_unsatisfiable_tag_fails() {
    let topo = small_topology();
    let intent = IntentBuilder::new()
        .name("impossible")
        .require_tag("nonexistent-tag")
        .build();
    let result = compile(&topo, &intent);
    assert!(result.is_err(), "should fail when no node matches required tag");
}
