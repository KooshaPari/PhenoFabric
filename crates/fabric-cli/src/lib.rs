//! Phenotype Fabric reference surface CLI.
//!
//! PF-WP-020.06 deliverable — primary user-facing interface for Fabric.
//!
//! Subcommands:
//!   cap      — capability probe, sign, verify, export, import
//!   graph    — topology build, show, add-node, add-edge
//!   route    — compile, plan, validate, list
//!   workspace — create, list, show, delete, release
//!   surface  — lease, list
//!   probe    — local machine capability probe (top-level)
//!   status   — daemon health check
//!   check    — run checker against manifest
//!   tui      — interactive TUI explorer

pub mod completions;
pub mod commands;
pub mod tui;
pub mod wire_client;
mod output;

use clap::Parser;

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
    /// Manage WorkOS authentication.
    Auth {
        #[command(subcommand)]
        sub: AuthCommand,
    },
    /// Manage named workspaces.
    Workspace {
        #[command(subcommand)]
        sub: WorkspaceCommand,
    },
    /// Query network status (UPnP, STUN, Tailscale).
    Network {
        #[command(subcommand)]
        sub: NetworkCommand,
    },
    /// Manage surface leases (displays, audio, network endpoints).
    Surface {
        #[command(subcommand)]
        sub: SurfaceCommand,
    },
    /// Probe local machine capabilities (top-level, outputs JSON).
    Probe(commands::probe::ProbeArgs),
    /// Show daemon health status.
    Status(commands::status::StatusArgs),
    /// Run checker against a manifest.
    Check(commands::check::CheckArgs),
    /// Print Fabric workspace version information.
    Version,
    /// Launch the interactive TUI topology explorer.
    Tui,
    /// Generate shell completion scripts.
    Completions(commands::completions::CompletionsArgs),
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
    /// Compile a route plan from topology + intent.
    Compile(commands::route::CompileArgs),
    /// List active route plans from the daemon.
    List(commands::route::ListArgs),
    /// Plan a sequence of route steps.
    Plan(commands::route::PlanArgs),
    /// Validate a compiled route plan.
    Validate(commands::route::ValidateArgs),
    /// Show a saved route plan.
    Show(commands::route::ShowArgs),
}

#[derive(Parser, Debug)]
pub enum AuthCommand {
    /// Initiate WorkOS browser login flow.
    Login(commands::auth::LoginArgs),
    /// Show current authentication status.
    Status(commands::auth::StatusArgs),
    /// Clear stored authentication tokens.
    Logout(commands::auth::LogoutArgs),
}

#[derive(Parser, Debug)]
pub enum WorkspaceCommand {
    /// Create a new named workspace.
    Create(commands::workspace::CreateArgs),
    /// List all workspaces.
    List(commands::workspace::ListArgs),
    /// Show details of a workspace with seat leases.
    Show(commands::workspace::ShowArgs),
    /// Delete a workspace.
    Delete(commands::workspace::DeleteArgs),
    /// Release a workspace and all its seat leases.
    Release(commands::workspace::ReleaseArgs),
}

#[derive(Parser, Debug)]
pub enum NetworkCommand {
    /// Show UPnP/STUN/Tailscale network status.
    Status(commands::network::NetworkStatusArgs),
    /// Query STUN server for external address.
    Stun(commands::network::StunArgs),
    /// Show Tailscale peer list.
    Tailscale(commands::network::TailscaleArgs),
}

#[derive(Parser, Debug)]
pub enum SurfaceCommand {
    /// Request a surface lease (display, audio, network endpoint).
    Lease(commands::surface::LeaseArgs),
    /// List active surface leases from the daemon.
    List(commands::surface::ListArgs),
}
