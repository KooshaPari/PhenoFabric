//! `fabric graph` — topology graph build / inspect commands.
//!
//! PF-WP-020.02: builds a CapabilityDescriptor graph from a topology
//! description (YAML/JSON) and validates it against the topology schema.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Subcommand;
use fabric_graph::model::{CapabilityRef, Edge, EdgeId, Node, NodeId, Topology, TopologyMeta};
use fabric_graph::score::score_intent;
use fabric_graph::model::Intent;
use fabric_graph::model::IntentRequirements;
use fabric_graph::builder::IntentBuilder;
use fabric_graph::negotiation::negotiate;

use crate::output::{self, OutputFormat};

#[derive(Subcommand, Debug)]
pub enum GraphCommand {
    /// Build a sample topology and emit it as JSON
    Sample,

    /// Load a topology from a JSON or YAML file and validate it
    Load {
        /// Path to the topology file
        path: PathBuf,
    },

    /// Print a brief summary of a topology
    Inspect {
        /// Path to the topology file
        path: PathBuf,
    },
}

impl GraphCommand {
    pub fn run(self, format: OutputFormat) -> Result<()> {
        match self {
            Self::Sample => run_sample(format),
            Self::Load { path } => run_load(&path, format),
            Self::Inspect { path } => run_inspect(&path, format),
        }
    }
}

fn run_sample(format: OutputFormat) -> Result<()> {
    let topo = sample_topology();
    if format.is_json() {
        println!("{}", serde_json::to_string_pretty(&topo)?);
    } else {
        output::print_topology_summary(&topo);
    }
    Ok(())
}

fn run_load(path: &Path, format: OutputFormat) -> Result<()> {
    let s = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let topo: Topology = parse_any(&s)
        .with_context(|| format!("parse topology {}", path.display()))?;
    if format.is_json() {
        println!("{}", serde_json::to_string_pretty(&topo)?);
    } else {
        output::print_topology_summary(&topo);
    }
    Ok(())
}

fn run_inspect(path: &Path, _format: OutputFormat) -> Result<()> {
    let s = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let topo: Topology = parse_any(&s).context("parse topology")?;
    output::print_topology_summary(&topo);
    Ok(())
}

fn parse_any(s: &str) -> Result<Topology> {
    if s.trim_start().starts_with('{') {
        Ok(serde_json::from_str(s)?)
    } else {
        Ok(serde_yaml::from_str(s)?)
    }
}

/// Build a small 2-node sample topology for the `sample` command and tests.
pub fn sample_topology() -> Topology {
    let mut topo = Topology::with_meta(TopologyMeta {
        schema_version: 1,
        source: "fabric-cli sample".into(),
        trust: fabric_capability::TrustLevel::SelfReported,
        ..Default::default()
    });

    let mut gpu = Node::new(NodeId::new("gpu-0"));
    gpu.add_capability(CapabilityRef::gpu("nvidia-rtx-4090"));
    gpu.add_tag("cuda");
    gpu.add_tag("host");
    gpu.add_metric("memory_gb", 24.0);
    topo.add_node(gpu);

    let mut cpu = Node::new(NodeId::new("cpu-0"));
    cpu.add_capability(CapabilityRef::cpu(16, 32_000));
    cpu.add_tag("host");
    topo.add_node(cpu);

    let edge = Edge::new(
        EdgeId::new("gpu-0->cpu-0"),
        NodeId::new("gpu-0"),
        NodeId::new("cpu-0"),
    )
    .with_latency_us(50)
    .with_bandwidth_mbps(10_000);
    topo.add_edge(edge);

    topo
}

/// Build a sample intent for the route planner.
pub fn sample_intent() -> Intent {
    IntentBuilder::new("demo-intent")
        .name("demo intent")
        .require_tag_for("compute", "cuda")
        .require_min_cores(8)
        .require_min_ram(8_000)
        .set_latency_budget(10_000)
        .build()
}

/// Score the sample topology against the sample intent (helper for tests / docs).
#[allow(dead_code)]
pub fn demo_negotiation(topo: &Topology, intent: &Intent) {
    let scored = score_intent(intent, topo);
    let result = negotiate(intent, topo, &scored);
    println!("{} candidates", result.candidates.len());
}

/// Re-export `IntentRequirements` so external callers can construct one directly.
pub use fabric_graph::model::IntentRequirements as GraphIntentRequirements;
