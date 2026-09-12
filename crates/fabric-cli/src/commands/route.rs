//! `fabric route` subcommand.

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

#[derive(Args, Debug)]
pub struct CompileArgs {
    #[arg(short, long)]
    pub topology: PathBuf,
    #[arg(long)]
    pub intent: String,
    #[arg(short, long, num_args = 1..)]
    pub require_tag: Vec<String>,
    #[arg(long, default_value_t = 0)]
    pub min_cores: u32,
    #[arg(long, default_value_t = 0)]
    pub min_ram: u32,
    #[arg(long, default_value_t = 0)]
    pub min_bandwidth: u64,
    #[arg(long, default_value_t = 8.0)]
    pub max_locality: f64,
    #[arg(long, default_value = "untrusted")]
    pub trust: String,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct PlanArgs {
    #[arg(short, long)]
    pub topology: PathBuf,
    #[arg(short, long, num_args = 2..)]
    pub intents: Vec<String>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    #[arg(short, long)]
    pub input: PathBuf,
}

#[derive(Args, Debug)]
pub struct ShowArgs {
    #[arg(short, long)]
    pub input: PathBuf,
}

pub fn dispatch(sub: &crate::RouteCommand, workspace: &Path) -> Result<()> {
    match sub {
        crate::RouteCommand::Compile(a) => compile_route(a, workspace),
        crate::RouteCommand::Plan(a) => plan_sequence(a, workspace),
        crate::RouteCommand::Validate(a) => validate(a),
        crate::RouteCommand::Show(a) => show(a),
    }
}

fn compile_route(args: &CompileArgs, workspace: &Path) -> Result<()> {
    let topology_json = std::fs::read_to_string(&args.topology)
        .with_context(|| format!("read {}", args.topology.display()))?;
    let topology: fabric_graph::model::Topology = serde_json::from_str(&topology_json)
        .context("parse topology")?;

    let trust = parse_trust(&args.trust)?;
    let mut builder = IntentBuilder::new()
        .name(&args.intent)
        .min_trust(trust)
        .max_locality(args.max_locality);
    for tag in &args.require_tag {
        builder = builder.require_tag(tag);
    }
    let intent = builder.build();

    let plan = compile(&topology, &intent).context("compile failed")?;

    let out_path = args.output.clone().unwrap_or_else(|| {
        workspace.join("routes").join(format!("{}.json", args.intent))
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

#[allow(dead_code)]
fn _unused_output(_o: &output::OutputFormat) {}
