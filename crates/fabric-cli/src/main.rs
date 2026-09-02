//! Phenotype Fabric reference surface CLI.
//!
//! PF-WP-020.06 deliverable — primary user-facing interface for Fabric.
//!
//! Subcommands:
//!   cap     — capability probe, sign, verify, export, import
//!   graph   — topology build, show, add-node, add-edge
//!   route   — compile, plan, validate
//!   workspace — create, list, show, delete

mod commands;
mod output;

use anyhow::Context;
use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(
    name = "fabric",
    version,
    about = "Phenotype Fabric reference surface CLI",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Suppress all output except errors.
    #[arg(short, long)]
    pub quiet: bool,

    /// Enable verbose output (vv for trace-level).
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Path to the workspace directory (default: ~/.fabric/).
    #[arg(short, long)]
    pub workspace: Option<std::path::PathBuf>,
}

#[derive(Parser, Debug)]
pub enum Commands {
    /// Probe and inspect machine capabilities.
    Cap {
        #[command(subcommand)]
        sub: CapCommand,
    },
    /// Build and inspect the capability topology graph.
    Graph {
        #[command(subcommand)]
        sub: GraphCommand,
    },
    /// Compile routes and manage route plans.
    Route {
        #[command(subcommand)]
        sub: RouteCommand,
    },
    /// Manage named workspaces.
    Workspace {
        #[command(subcommand)]
        sub: WorkspaceCommand,
    },
}

#[derive(Parser, Debug)]
pub enum CapCommand {
    /// Probe local machine capabilities and emit a CapabilityDescriptor.
    Probe(commands::cap::ProbeArgs),
    /// Sign a capability descriptor with a local Ed25519 key.
    Sign(commands::cap::SignArgs),
    /// Verify a signed capability descriptor.
    Verify(commands::cap::VerifyArgs),
    /// Export a capability descriptor to JSON.
    Export(commands::cap::ExportArgs),
    /// Import a capability descriptor from NVMS manifest.
    ImportNvms(commands::cap::ImportNvmsArgs),
    /// Validate a descriptor against the JSON schema.
    Validate(commands::cap::ValidateArgs),
}

#[derive(Parser, Debug)]
pub enum GraphCommand {
    /// Build a topology graph from one or more capability descriptors.
    Build(commands::graph::BuildArgs),
    /// Show a saved topology graph.
    Show(commands::graph::ShowArgs),
    /// Add a node to a topology graph.
    AddNode(commands::graph::AddNodeArgs),
    /// Add an edge between two nodes in a topology graph.
    AddEdge(commands::graph::AddEdgeArgs),
}

#[derive(Parser, Debug)]
pub enum RouteCommand {
    /// Compile a route plan for an intent.
    Compile(commands::route::CompileArgs),
    /// Plan a sequence of route steps.
    Plan(commands::route::PlanArgs),
    /// Validate a compiled route plan.
    Validate(commands::route::ValidateArgs),
    /// Show a saved route plan.
    Show(commands::route::ShowArgs),
}

#[derive(Parser, Debug)]
pub enum WorkspaceCommand {
    /// Create a new named workspace.
    Create(commands::workspace::CreateArgs),
    /// List all workspaces.
    List(commands::workspace::ListArgs),
    /// Show details of a workspace.
    Show(commands::workspace::ShowArgs),
    /// Delete a workspace.
    Delete(commands::workspace::DeleteArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = match cli.verbose {
        0 => EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("warn")),
        1 => EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info")),
        2 => EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("debug")),
        _ => EnvFilter::new("trace"),
    };

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_level(true))
        .with(filter)
        .init();

    // Resolve workspace path
    let workspace = cli.workspace.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".fabric")
    });

    // Dispatch
    let result = match &cli.command {
        Commands::Cap { sub } => commands::cap::dispatch(sub, &workspace),
        Commands::Graph { sub } => commands::graph::dispatch(sub, &workspace),
        Commands::Route { sub } => commands::route::dispatch(sub, &workspace),
        Commands::Workspace { sub } => commands::workspace::dispatch(sub, &workspace),
    };

    if let Err(ref e) = result {
        if !cli.quiet {
            eprintln!("{}: {}", console::style("error").red().bold(), e);
            for cause in std::iter::successors(e.source(), |e| e.source()) {
                eprintln!("  {}: cause: {}", console::style("↳").dim(), cause);
            }
        }
        std::process::exit(1);
    }

    Ok(())
}
