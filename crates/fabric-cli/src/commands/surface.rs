//! `fabric surface` subcommand.
//!
//! Manages surface leases — the user-facing presentation bindings that
//! connect route plans to concrete capability surfaces (displays, audio,
//! network endpoints, etc.).

use anyhow::{Context, Result};
use clap::Args;
use std::path::Path;
use std::str::FromStr;

use fabric_graph::surface::{SurfaceProtocol, SurfaceSpec};

use crate::output;
use crate::wire_client;

#[derive(Args, Debug)]
pub struct LeaseArgs {
    /// Presentation protocol (posix, vnc, rdp, webrtc, asp).
    #[arg(short, long)]
    pub protocol: String,
    /// Human-readable name for the surface (e.g. "primary-display", "headphones").
    #[arg(short, long)]
    pub name: String,
    /// Locality floor (e.g. "L0SameProcess", "L5Loopback"). Default: L5Loopback.
    #[arg(long, default_value = "L5Loopback")]
    pub locality_floor: String,
    /// Whether the surface requires a real-time thread.
    #[arg(long)]
    pub rt: bool,
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
    /// Daemon address (default: 127.0.0.1:9400).
    #[arg(long, default_value = wire_client::DEFAULT_DAEMON_ADDR)]
    pub daemon: String,
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

pub fn dispatch(sub: &crate::SurfaceCommand, workspace: &Path) -> Result<()> {
    match sub {
        crate::SurfaceCommand::Lease(a) => lease(a, workspace),
        crate::SurfaceCommand::List(a) => list(a),
    }
}

fn lease(args: &LeaseArgs, _workspace: &Path) -> Result<()> {
    // Parse the protocol string into a SurfaceProtocol.
    let protocol = parse_protocol(&args.protocol)?;

    // Parse locality tier.
    let locality = fabric_capability::locality::LocalityTier::from_str(&args.locality_floor)
        .map_err(|e| anyhow::anyhow!("unknown locality tier '{}': {}", args.locality_floor, e))?;

    // Build a SurfaceSpec.
    let spec = SurfaceSpec {
        name: args.name.clone(),
        protocol,
        capture: None,
        locality_floor: locality,
        refresh_hz: None,
        audio_sample_rate_hz: None,
        requires_rt_island: args.rt,
        strict_epoch_binding: false,
        min_host_trust: fabric_graph::TrustLevel::Untrusted,
        expires_at: None,
    };

    // Validate the spec.
    spec.validate()
        .map_err(|e| anyhow::anyhow!("invalid surface spec: {}", e))?;

    // Create a lease via the daemon wire protocol.
    let msg = serde_json::json!({
        "type": "surface_lease_request",
        "name": args.name,
        "protocol": args.protocol,
        "locality_floor": args.locality_floor,
        "requires_rt": args.rt,
    });

    match wire_client::send_message(&args.daemon, &msg) {
        Ok(response) => {
            let format = output::resolve_format(args.json, false);
            output::emit(format, &response, || {
                let handle = response
                    .get("handle")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let state = response
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                format!(
                    "{} surface lease {} ({})\n",
                    console::style("leased").green().bold(),
                    handle,
                    state,
                )
            })?;
        }
        Err(wire_client::WireClientError::ConnectionRefused { addr }) => {
            // Daemon not running — report the lease spec that would be created.
            eprintln!(
                "{} daemon not reachable at {} — surface spec validated but not yet leased",
                console::style("warning:").yellow().bold(),
                addr,
            );
            let handle = uuid::Uuid::now_v7().to_string();
            let result = serde_json::json!({
                "handle": handle,
                "state": "pending",
                "spec": {
                    "name": args.name,
                    "protocol": args.protocol,
                    "locality_floor": args.locality_floor,
                    "requires_rt": args.rt,
                },
            });
            let format = output::resolve_format(args.json, false);
            output::emit(format, &result, || {
                format!(
                    "{} surface lease {} ({})\n  {} daemon not running — lease is pending\n",
                    console::style("leased").yellow().bold(),
                    handle,
                    "pending",
                    console::style("warning:").yellow(),
                )
            })?;
        }
        Err(e) => return Err(e).context("surface lease request failed"),
    }

    Ok(())
}

fn list(args: &ListArgs) -> Result<()> {
    let msg = serde_json::json!({"type": "capabilities_request"});

    match wire_client::send_message(&args.daemon, &msg) {
        Ok(response) => {
            let format = output::resolve_format(args.json, false);
            output::emit(format, &response, || {
                let caps = response
                    .get("capabilities")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                let mut out = format!(
                    "{} active surface capabilities (via daemon)\n\n",
                    console::style(caps).cyan().bold(),
                );
                if let Some(caps_arr) = response.get("capabilities").and_then(|v| v.as_array()) {
                    out.push_str(&format!(
                        "{:<24} {:<36} {}\n",
                        "NODE", "DESCRIPTOR", "TRUST"
                    ));
                    for cap in caps_arr {
                        let node = cap.get("node_name").and_then(|v| v.as_str()).unwrap_or("?");
                        let desc = cap
                            .get("descriptor_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let trust = cap.get("trust").and_then(|v| v.as_str()).unwrap_or("?");
                        out.push_str(&format!("{:<24} {:<36} {}\n", node, desc, trust));
                    }
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
                        "surfaces": [],
                    })
                );
            } else {
                eprintln!(
                    "{} daemon not reachable at {}",
                    console::style("error:").red().bold(),
                    addr,
                );
                eprintln!("  start fabric-daemon to manage surface leases");
            }
        }
        Err(e) => return Err(e).context("surface list request failed"),
    }

    Ok(())
}

fn parse_protocol(s: &str) -> Result<SurfaceProtocol> {
    match s.to_lowercase().as_str() {
        "posix" => Ok(SurfaceProtocol::Posix),
        "vnc" => Ok(SurfaceProtocol::Vnc),
        "rdp" => Ok(SurfaceProtocol::Rdp),
        "webrtc" | "web_rtc" | "web-rtc" => Ok(SurfaceProtocol::WebRtc),
        "asp" => Ok(SurfaceProtocol::Asp),
        other => Ok(SurfaceProtocol::Custom(other.to_string())),
    }
}
