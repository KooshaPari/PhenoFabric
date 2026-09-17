//! SSH tunnel management for cross-machine tmux access.
//!
//! Connects to remote machine via SSH, executes tmux commands,
//! and captures screen content. Uses ssh2 for the transport.

use anyhow::{Context, Result};
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use tracing::{debug, info};

use crate::PaneContent;

/// SSH tunnel to a remote tmux server.
pub struct SshTunnel {
    session: Session,
    socket_path: String,
}

impl SshTunnel {
    /// Connect to a remote machine via SSH.
    ///
    /// `target` format: "user@host" or just "host" (uses current user).
    /// `socket_path` is the tmux socket path on the remote.
    pub async fn connect(target: &str, socket_path: &str) -> Result<Self> {
        info!("Connecting to {} via SSH", target);

        // Parse target into host and optional user
        let (user, host) = if let Some((u, h)) = target.split_once('@') {
            (Some(u), h)
        } else {
            (None, target)
        };

        // Resolve host to IP
        let addr = format!("{}:22", host);
        let tcp = TcpStream::connect(&addr)
            .with_context(|| format!("Failed to connect to {}:22", host))?;
        tcp.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        // Authenticate
        if let Some(user) = user {
            session.userauth_agent(user)?;
        } else {
            // Use current user
            let current_user = std::env::var("USER").unwrap_or_default();
            session.userauth_agent(&current_user)?;
        }

        if !session.authenticated() {
            anyhow::bail!("SSH authentication failed");
        }

        info!("SSH connected and authenticated");

        Ok(Self {
            session,
            socket_path: socket_path.to_string(),
        })
    }

    /// Execute a tmux command on the remote machine.
    pub async fn exec_tmux(&self, command: &str) -> Result<String> {
        let full_cmd = format!("tmux -S {} {}", self.socket_path, command);

        debug!("Executing: {}", full_cmd);

        let mut channel = self.session.channel_session()?;
        channel.exec(&full_cmd)?;

        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;

        Ok(output.trim().to_string())
    }

    /// List all panes on the remote tmux server.
    pub async fn list_panes(&self) -> Result<Vec<PaneInfo>> {
        let output = self
            .exec_tmux("list-panes -F \"#{pane_id}|#{window_name}|#{pane_width}|#{pane_height}|#{pane_alive}\"")
            .await?;

        let mut panes = Vec::new();
        for line in output.lines() {
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('|').collect();
            if fields.len() >= 5 {
                panes.push(PaneInfo {
                    pane_id: fields[0].to_string(),
                    window_name: fields[1].to_string(),
                    width: fields[2].parse().unwrap_or(0),
                    height: fields[3].parse().unwrap_or(0),
                    alive: fields[4] != "0",
                });
            }
        }

        Ok(panes)
    }

    /// Capture a pane's screen content.
    pub async fn capture_pane(&self, pane_id: &str) -> Result<PaneContent> {
        let output = self
            .exec_tmux(&format!("capture-pane -t {} -p", pane_id))
            .await?;

        let lines: Vec<String> = output.lines().map(|l| l.to_string()).collect();
        let height = lines.len() as u32;

        // Get cursor position
        let cursor_output = self
            .exec_tmux(&format!(
                "display-message -t {} -p \"#{{cursor_y}},#{{cursor_x}},#{{pane_width}},#{{pane_height}}\"",
                pane_id
            ))
            .await?;

        let cursor_parts: Vec<&str> = cursor_output.split(',').collect();
        let cursor_row = cursor_parts
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let cursor_col = cursor_parts
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let width = cursor_parts
            .get(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(80);

        Ok(PaneContent {
            pane_id: pane_id.to_string(),
            lines,
            width,
            height,
            cursor_row,
            cursor_col,
            dirty: true,
        })
    }
}

/// Pane information from tmux.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaneInfo {
    pub pane_id: String,
    pub window_name: String,
    pub width: u32,
    pub height: u32,
    pub alive: bool,
}
