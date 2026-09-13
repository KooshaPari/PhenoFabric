//! `fabric route` subcommand.
//!
//! Compile routes from topology + intent, list active routes from daemon,
//! validate and show saved route plans.

use anyhow::{Context, Result};
use clap::Args;
use std::path::{Path, PathBuf};

use fabric_graph::{
    builder::IntentBuilder,
    compile,
    model::TrustLevel,
    planner,
};

use crate::output;
use crate::wire_client;

#[derive(Args, Debug)]
pub struct CompileArgs {
    /// Path to the topology JSON file.
    #[arg(short, long)]
    pub topology: PathBuf,
    /// Path to the intent JSON file (or inline intent name).
    #[arg(short, long)]
    pub intent: String,
    /// Required tags for route selection.
    #[arg(short, long, num_args = 1..)]
    pub require_tag: Vec<String>,
    /// Minimum CPU cores required.
    #[arg(long, default_value_t = 0)]
    pub min_cores: u32,
    /// Minimum RAM in bytes required.
    #[arg(long, default_value_t = 0)]
    pub min_ram: u32,
    /// Minimum bandwidth in bps required.
    #[arg(long, default_value_t = 0)]
    pub min_bandwidth: u64,
    /// Maximum locality tier (default: 8.0).
    #[arg(long, default_value_t = 8.0)]
    pub max_locality: f64,
    /// Minimum trust level (untrusted, bootstrap, attested, audited).
    #[arg(long, default_value = "untrusted")]
    pub trust: String,
    /// Output file path.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
    /// Daemon address (default: 127.0.0.1:9400).
    #[arg(long, default_value = wire_client::DEFAULT_DAEMON_ADDR)]
    pub daemon: String,
}

#[derive(Args, Debug)]
pub struct PlanArgs {
    /// Path to the topology JSON file.
    #[arg(short, long)]
    pub topology: PathBuf,
    /// Intent names for the sequence.
    #[arg(short, long, num_args = 2..)]
    pub intents: Vec<String>,
    /// Output file path.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Path to the route plan JSON file.
    #[arg(short, long)]
    pub input: PathBuf,
}

#[derive(Args, Debug)]
pub struct ShowArgs {
    /// Path to the route plan JSON file.
    #[arg(short, long)]
    pub input: PathBuf,
}

pub fn dispatch(sub: &crate::RouteCommand, workspace: &Path) -> Result<()> {
    match sub {
        crate::RouteCommand::Compile(a) => compile_route(a, workspace),
        crate::RouteCommand::List(a) => list(a),
        crate::RouteCommand::Plan(a) => plan_sequence(a, workspace),
        crate::RouteCommand::Validate(a) => validate(a),
        crate::RouteCommand::Show(a) => show(a),
    }
}

fn compile_route(args: &CompileArgs, workspace: &Path) -> Result<()> {
    // Load topology.
    let topology_json = std::fs::read_to_string(&args.topology)
        .with_context(|| format!("read {}", args.topology.display()))?;
    let topology: fabric_graph::model::Topology = serde_json::from_str(&topology_json)
        .context("parse topology")?;

    // Load intent: try as file path first, fall back to inline name.
    let intent = if Path::new(&args.intent).exists() {
        let intent_json = std::fs::read_to_string(&args.intent)
            .with_context(|| format!("read intent file {}", args.intent))?;
        // Try to parse as a full Intent, or as a simple {name: "..."} object.
        if let Ok(full_intent) = serde_json::from_str::<fabric_graph::model::Intent>(&intent_json) {
            full_intent
        } else {
            let obj: serde_json::Value = serde_json::from_str(&intent_json)
                .context("parse intent JSON")?;
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&args.intent);
            let trust = parse_trust(
                obj.get("trust")
                    .and_then(|v| v.as_str())
                    .unwrap_or("untrusted"),
            )?;
            let max_locality = obj
                .get("max_locality")
                .and_then(|v| v.as_f64())
                .unwrap_or(args.max_locality);
            let mut builder = IntentBuilder::new()
                .name(name)
                .min_trust(trust)
                .max_locality(max_locality);
            if let Some(tags) = obj.get("tags").and_then(|v| v.as_array()) {
                for tag in tags {
                    if let Some(t) = tag.as_str() {
                        builder = builder.require_tag(t);
                    }
                }
            }
            builder.build()
        }
    } else {
        // Treat as an inline intent name.
        let trust = parse_trust(&args.trust)?;
        let mut builder = IntentBuilder::new()
            .name(&args.intent)
            .min_trust(trust)
            .max_locality(args.max_locality);
        for tag in &args.require_tag {
            builder = builder.require_tag(tag);
        }
        builder.build()
    };

    let plan = compile(&topology, &intent).context("compile failed")?;

    let out_path = args.output.clone().unwrap_or_else(|| {
        workspace.join("routes").join(format!("{}.json", intent.name))
    });
    let json = serde_json::to_string_pretty(&plan)?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out_path, &json)
        .with_context(|| format!("write {}", out_path.display()))?;
    eprintln!("wrote {}", out_path.display());

    if args.json {
        println!("{}", json);
    } else {
        println!(
            "{} {:?}",
            console::style("Route plan:").cyan().bold(),
            plan.id
        );
        println!("  total steps: {}", plan.steps.len());
        if let Some(ref score) = plan.score {
            println!("  score:       {:.3}", score.composite);
        }
        for (i, step) in plan.steps.iter().enumerate() {
            println!(
                "  step {}: node={}",
                i,
                step.node,
            );
        }
    }
    Ok(())
}

fn list(args: &ListArgs) -> Result<()> {
    let msg = serde_json::json!({"type": "routes_request"});

    match wire_client::send_message(&args.daemon, &msg) {
        Ok(response) => {
            let format = output::resolve_format(args.json, false);
            output::emit(format, &response, || {
                let routes = response
                    .get("routes")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let count = routes.len();
                let mut out = format!(
                    "{} active route plans (via daemon)\n\n",
                    console::style(count).cyan().bold(),
                );
                out.push_str(&format!(
                    "{:<36} {:<8} {:<14} {:<10} {}\n",
                    "INTENT_ID", "STEPS", "EPOCH", "LATENCY_US", "TAGS"
                ));
                for route in &routes {
                    let intent_id = route
                        .get("intent_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let steps = route
                        .get("steps")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let epoch = route
                        .get("topology_epoch")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let latency = route
                        .get("estimated_latency_us")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let tags = route
                        .get("tags")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(",")
                        })
                        .unwrap_or_default();
                    out.push_str(&format!(
                        "{:<36} {:<8} {:<14} {:<10.1} {}\n",
                        intent_id, steps, epoch, latency, tags
                    ));
                }
                out
            })?;
        }
        Err(wire_client::WireClientError::ConnectionRefused { addr }) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "error": "daemon_unreachable",
                        "address": addr,
                        "routes": [],
                    })
                );
            } else {
                eprintln!(
                    "{} daemon not reachable at {}",
                    console::style("error:").red().bold(),
                    addr,
                );
                eprintln!("  start fabric-daemon to list active routes");
            }
        }
        Err(e) => return Err(e).context("route list failed"),
    }

    Ok(())
}

fn plan_sequence(args: &PlanArgs, workspace: &Path) -> Result<()> {
    let topology_json = std::fs::read_to_string(&args.topology)
        .with_context(|| format!("read {}", args.topology.display()))?;
    let topology: fabric_graph::model::Topology = serde_json::from_str(&topology_json)
        .context("parse topology")?;
    let intents: Vec<_> = args
        .intents
        .iter()
        .map(|name| {
            IntentBuilder::new()
                .name(name)
                .build()
        })
        .collect();
    let plan = planner::plan_sequence(&topology, "cli-sequence", &intents)
        .context("plan_sequence failed")?;
    let out_path = args.output.clone().unwrap_or_else(|| {
        workspace
            .join("routes")
            .join(format!("plan-{}.json", uuid::Uuid::now_v7()))
    });
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(&plan)?;
    std::fs::write(&out_path, json)
        .with_context(|| format!("write {}", out_path.display()))?;
    println!(
        "wrote plan with {} routes to {}",
        plan.plans.len(),
        out_path.display()
    );
    Ok(())
}

fn validate(args: &ValidateArgs) -> Result<()> {
    let json = std::fs::read_to_string(&args.input)
        .with_context(|| format!("read {}", args.input.display()))?;
    let _plan: fabric_graph::model::RoutePlan = serde_json::from_str(&json)
        .context("parse route plan JSON")?;
    println!("OK: {} is a valid RoutePlan", args.input.display());
    Ok(())
}

fn show(args: &ShowArgs) -> Result<()> {
    let json = std::fs::read_to_string(&args.input)
        .with_context(|| format!("read {}", args.input.display()))?;
    let plan: fabric_graph::model::RoutePlan = serde_json::from_str(&json)
        .context("parse route plan JSON")?;
    println!("{}", console::style("Route plan:").cyan().bold());
    println!("  id:    {:?}", plan.id);
    println!("  steps: {}", plan.steps.len());
    if let Some(ref score) = plan.score {
        println!("  score: {:.3}", score.composite);
    }
    for (i, step) in plan.steps.iter().enumerate() {
        println!(
            "  step {}: node={}",
            i,
            step.node,
        );
    }
    Ok(())
}

fn parse_trust(s: &str) -> Result<TrustLevel> {
    match s {
        "untrusted" => Ok(TrustLevel::Untrusted),
        "bootstrap" => Ok(TrustLevel::Bootstrap),
        "attested" => Ok(TrustLevel::Attested),
        "audited" => Ok(TrustLevel::Audited),
        other => Err(anyhow::anyhow!("unknown trust level: {}", other)),
    }
}
