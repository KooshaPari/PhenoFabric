//! Tests for fabric CLI graph subcommand.
//!
//! Builds topology from descriptors and validates add-node, add-edge.

use std::fs;
use std::path::PathBuf;

use fabric_graph::builder::TopologyBuilder;
use fabric_graph::model::{EdgeId, Node, NodeId, Edge};
use fabric_capability::locality::LocalityTier;

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "fabric-cli-graph-{}-{}",
        std::process::id(),
        uuid::Uuid::now_v7()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn build_topology_from_nodes() {
    let mut builder = TopologyBuilder::new().with_name("test-topo");
    let n1 = Node::new(NodeId::new("node-a"), LocalityTier::L0SameProcess);
    let n2 = Node::new(NodeId::new("node-b"), LocalityTier::L6Lan);
    builder = builder.add(n1).add(n2);
    let topo = builder.build();
    assert_eq!(topo.node_count(), 2);
    assert_eq!(topo.edge_count(), 0);
}

#[test]
fn topology_save_and_load_roundtrip() {
    let dir = temp_dir();
    let mut builder = TopologyBuilder::new().with_name("roundtrip-test");
    let n1 = Node::new(NodeId::new("host-1"), LocalityTier::L0SameProcess);
    let n2 = Node::new(NodeId::new("host-2"), LocalityTier::L6Lan);
    builder = builder.add(n1).add(n2);
    let topo = builder.build();

    let path = dir.join("topo.json");
    let json = serde_json::to_string_pretty(&topo).unwrap();
    fs::write(&path, &json).unwrap();

    let loaded: fabric_graph::model::Topology = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.node_count(), 2);
    assert_eq!(loaded.meta.name, "roundtrip-test");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn add_node_to_topology() {
    let mut builder = TopologyBuilder::new().with_name("add-node-test");
    let n1 = Node::new(NodeId::new("existing"), LocalityTier::L0SameProcess);
    builder = builder.add(n1);
    let mut topo = builder.build();

    let n2 = Node::new(NodeId::new("new-node"), LocalityTier::L6Lan);
    topo.add_node(n2);
    assert_eq!(topo.node_count(), 2);
}

#[test]
fn add_edge_to_topology() {
    let mut builder = TopologyBuilder::new().with_name("add-edge-test");
    let n1 = Node::new(NodeId::new("a"), LocalityTier::L0SameProcess);
    let n2 = Node::new(NodeId::new("b"), LocalityTier::L6Lan);
    builder = builder.add(n1).add(n2);
    let mut topo = builder.build();

    let edge = Edge::new(
        EdgeId::new("a-b"),
        NodeId::new("a"),
        NodeId::new("b"),
        LocalityTier::L6Lan,
    );
    topo.add_edge(edge);
    assert_eq!(topo.edge_count(), 1);
}
