//! `fabric graph` subcommand.

use anyhow::{Context, Result};
use clap::Args;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use fabric_capability::descriptor::CapabilityDescriptor;
use fabric_capability::locality::LocalityTier;
use fabric_graph::builder::TopologyBuilder;
use fabric_graph::model::{CapabilityRef, Edge, LinkMetrics, Node, NodeId, Topology};

use crate::output;

#[derive(Args, Debug)]
pub struct BuildArgs {
    #[arg(short, long)]
    pub name: String,
    #[arg(short, long, num_args = 1..)]
    pub descriptors: Vec<PathBuf>,
    #[arg(short, long)]
    pub label: Option<String>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ShowArgs {
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct AddNodeArgs {
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(short, long)]
    pub node_id: String,
    #[arg(long)]
    pub tier: String,
    #[arg(short, long)]
    pub label: Option<String>,
    #[arg(short, long)]
    pub descriptor: Option<PathBuf>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct AddEdgeArgs {
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(long)]
    pub from: String,
    #[arg(long)]
    pub to: String,
    #[arg(long, default_value_t = 1_000_000_000)]
    pub bandwidth: u64,
    #[arg(long, default_value_t = 1_000)]
    pub max_latency_us: u32,
    #[arg(long, default_value_t = 0.0)]
    pub loss: f64,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

pub fn dispatch(sub: &crate::GraphCommand, workspace: &Path) -> Result<()> {
    match sub {
        crate::GraphCommand::Build(a) => build(a, workspace),
        crate::GraphCommand::Show(a) => show(a),
        crate::GraphCommand::AddNode(a) => add_node(a),
        crate::GraphCommand::AddEdge(a) => add_edge(a),
    }
}

fn build(args: &BuildArgs, workspace: &Path) -> Result<()> {
    let mut builder = TopologyBuilder::new().with_name(&args.name);
    for path in &args.descriptors {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        let desc: CapabilityDescriptor = serde_json::from_str(&json).context("parse descriptor")?;
        let node_id = NodeId::new(desc.node_id.to_string());
        let locality = LocalityTier::L0SameProcess;
        let cap_ref = CapabilityRef::new(desc.node_id.to_string())
            .with_trust(fabric_graph::model::TrustLevel::Untrusted);
        let mut node = Node::new(node_id, locality).with_capability(cap_ref);
        if let Some(label) = &args.label {
            node = node.with_label(label);
        }
        builder = builder.add(node);
    }
    let topology = builder.build();
    save_topology(&topology, args.output.as_deref(), workspace, &args.name)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&topology)?);
    } else {
        output::print_topology_summary(&args.name, topology.node_count(), topology.edge_count());
    }
    Ok(())
}

fn show(args: &ShowArgs) -> Result<()> {
    let topology = load_topology(&args.input)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&topology)?);
    } else {
        output::print_topology_summary(
            &topology.meta.name,
            topology.node_count(),
            topology.edge_count(),
        );
        println!("{:#?}", topology);
    }
    Ok(())
}

fn add_node(args: &AddNodeArgs) -> Result<()> {
    let mut topology = load_topology(&args.input)?;
    let tier = LocalityTier::from_str(&args.tier)
        .map_err(|e| anyhow::anyhow!("unknown locality tier '{}': {}", args.tier, e))?;
    let cap_ref = if let Some(desc_path) = &args.descriptor {
        let json = std::fs::read_to_string(desc_path)
            .with_context(|| format!("read {}", desc_path.display()))?;
        let desc: CapabilityDescriptor = serde_json::from_str(&json)?;
        Some(
            CapabilityRef::new(desc.node_id.to_string())
                .with_trust(fabric_graph::model::TrustLevel::Untrusted),
        )
    } else {
        None
    };
    let mut node = Node::new(NodeId::new(args.node_id.clone()), tier);
    if let Some(label) = &args.label {
        node = node.with_label(label);
    }
    if let Some(cr) = cap_ref {
        node = node.with_capability(cr);
    }
    topology.add_node(node);
    save_topology(
        &topology,
        args.output.as_deref().or(Some(args.input.as_path())),
        args.input.parent().unwrap_or(Path::new(".")),
        &topology.meta.name,
    )?;
    println!("added node {} to topology", args.node_id);
    Ok(())
}

fn add_edge(args: &AddEdgeArgs) -> Result<()> {
    let mut topology = load_topology(&args.input)?;
    let metrics = LinkMetrics {
        latency_us: Some(args.max_latency_us as f64),
        bandwidth_bps: Some(args.bandwidth),
        packet_loss: Some(args.loss),
        jitter_us: None,
    };
    let edge = Edge::new(
        fabric_graph::model::EdgeId::new(format!("{}-{}", args.from, args.to)),
        NodeId::new(args.from.clone()),
        NodeId::new(args.to.clone()),
        LocalityTier::L6Lan,
    )
    .with_metrics(metrics);
    topology.add_edge(edge);
    save_topology(
        &topology,
        args.output.as_deref().or(Some(args.input.as_path())),
        args.input.parent().unwrap_or(Path::new(".")),
        &topology.meta.name,
    )?;
    println!("added edge {} -> {}", args.from, args.to);
    Ok(())
}

fn load_topology(path: &Path) -> Result<Topology> {
    let json = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&json).context("parse topology JSON")
}

fn save_topology(
    topology: &Topology,
    output: Option<&Path>,
    workspace: &Path,
    name: &str,
) -> Result<()> {
    let out_path = match output {
        Some(p) => p.to_path_buf(),
        None => {
            let dir = workspace.join("topologies");
            std::fs::create_dir_all(&dir)?;
            dir.join(format!("{}.json", name))
        }
    };
    let json = serde_json::to_string_pretty(topology)?;
    std::fs::write(&out_path, json)
        .with_context(|| format!("write {}", out_path.display()))?;
    eprintln!("wrote {}", out_path.display());
    Ok(())
}
