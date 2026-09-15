//! tf-sync: Cross-machine terminal synchronization
//!
//! Manages SSH tunnels and screen content relay between machines.
//! Rust gives us memory-safe concurrency for network operations
//! and the best SSH library ecosystem.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod relay;
pub mod state;
#[cfg(feature = "ssh")]
pub mod ssh_tunnel;

use state::SyncState;



/// tf-sync: Cross-machine terminal synchronization
#[derive(Parser)]
#[command(name = "tf-sync", version, about)]
struct Cli {
    /// SSH target (user@host)
    #[arg(short, long)]
    target: Option<String>,

    /// Path to tmux socket on remote
    #[arg(short, long, default_value = "/private/tmp/tmux-501/default")]
    socket: String,

    /// Local tmux socket path
    #[arg(long)]
    local_socket: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start synchronization with a remote machine
    Sync {
        /// Remote tmux session name
        #[arg(short, long)]
        session: Option<String>,

        /// Pane IDs to sync (comma-separated, or "all")
        #[arg(short, long, default_value = "all")]
        panes: String,

        /// Poll interval in milliseconds
        #[arg(long, default_value = "500")]
        poll_ms: u64,
    },

    /// List panes on the remote machine
    ListPanes,

    /// Capture a remote pane's screen content
    Capture {
        /// Pane ID to capture
        pane_id: String,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Show sync status and connection info
    Status,
}

/// A single pane's screen content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneContent {
    pub pane_id: String,
    pub lines: Vec<String>,
    pub width: u32,
    pub height: u32,
    pub cursor_row: u32,
    pub cursor_col: u32,
    pub dirty: bool,
}

/// Sync state between two machines
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncSnapshot {
    pub source_machine: String,
    pub target_machine: String,
    pub panes: Vec<PaneContent>,
    pub timestamp: u64,
    pub sequence: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tf_sync=info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Sync {
            ref session,
            ref panes,
            poll_ms,
        } => {
            cmd_sync(&cli, session.as_deref(), &panes, poll_ms).await?;
        }
        Commands::ListPanes => {
            cmd_list_panes(&cli).await?;
        }
        Commands::Capture { ref pane_id, ref format } => {
            cmd_capture(&cli, &pane_id, &format).await?;
        }
        Commands::Status => {
            cmd_status(&cli).await?;
        }
    }

    Ok(())
}

async fn cmd_sync(cli: &Cli, session: Option<&str>, panes: &str, poll_ms: u64) -> Result<()> {
    let target = cli
        .target
        .as_deref()
        .context("SSH target required (--target)")?;

    tracing::info!(
        "Starting sync: target={} session={:?} panes={} poll={}ms",
        target,
        session,
        panes,
        poll_ms
    );

    // Establish SSH connection
    let tunnel = Arc::new(RwLock::new(
        ssh_tunnel::SshTunnel::connect(target, &cli.socket).await?
    ));

    let state = Arc::new(RwLock::new(SyncState::new()));

    let mut interval = tokio::time::interval(std::time::Duration::from_millis(poll_ms));

    loop {
        interval.tick().await;

        let screen = {
            let tunnel_guard = tunnel.read().await;
            tunnel_guard.capture_pane("0.0").await
        };

        match screen {
            Ok(content) => {
                let mut state = state.write().await;
                let changed = state.update(&content);

                if changed {
                    tracing::info!("Pane 0.0 changed ({} lines)", content.lines.len());
                    // Broadcast to local tmux
                    relay::broadcast_local(&content).await?;
                }
            }
            Err(e) => {
                tracing::warn!("Capture failed: {}", e);
                // Attempt reconnection
                match ssh_tunnel::SshTunnel::connect(target, &cli.socket).await {
                    Ok(new_tunnel) => {
                        let mut t = tunnel.write().await;
                        *t = new_tunnel;
                        tracing::info!("Reconnected to {}", target);
                    }
                    Err(re) => {
                        tracing::error!("Reconnect failed: {}", re);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }
}

async fn cmd_list_panes(cli: &Cli) -> Result<()> {
    let target = cli
        .target
        .as_deref()
        .context("SSH target required (--target)")?;

    let tunnel = ssh_tunnel::SshTunnel::connect(target, &cli.socket).await?;
    let panes = tunnel.list_panes().await?;

    let json = serde_json::to_string_pretty(&panes)?;
    println!("{}", json);

    Ok(())
}

async fn cmd_capture(cli: &Cli, pane_id: &str, format: &str) -> Result<()> {
    let target = cli
        .target
        .as_deref()
        .context("SSH target required (--target)")?;

    let tunnel = ssh_tunnel::SshTunnel::connect(target, &cli.socket).await?;
    let content = tunnel.capture_pane(pane_id).await?;

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&content)?;
            println!("{}", json);
        }
        "text" => {
            for line in &content.lines {
                println!("{}", line);
            }
        }
        _ => {
            anyhow::bail!("Unknown format: {}", format);
        }
    }

    Ok(())
}

async fn cmd_status(cli: &Cli) -> Result<()> {
    println!("tf-sync status");
    println!("  Target: {:?}", cli.target);
    println!("  Socket: {}", cli.socket);

    if let Some(target) = &cli.target {
        match ssh_tunnel::SshTunnel::connect(target, &cli.socket).await {
            Ok(tunnel) => {
                println!("  Connection: OK");
                let panes = tunnel.list_panes().await?;
                println!("  Panes: {}", panes.len());
            }
            Err(e) => {
                println!("  Connection: FAILED ({})", e);
            }
        }
    } else {
        println!("  Connection: No target specified");
    }

    Ok(())
}
