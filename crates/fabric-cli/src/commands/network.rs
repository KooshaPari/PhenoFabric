//! `fabric network` subcommand.
//!
//! Query and display network status — UPnP, STUN, and Tailscale.

use anyhow::{Context, Result};
use clap::Args;

use crate::output;
use crate::wire_client;

#[derive(Args, Debug)]
pub struct NetworkStatusArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
    /// Daemon address (default: 127.0.0.1:9400).
    #[arg(long, default_value = wire_client::DEFAULT_DAEMON_ADDR)]
    pub daemon: String,
}

#[derive(Args, Debug)]
pub struct StunArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
    /// Daemon address (default: 127.0.0.1:9400).
    #[arg(long, default_value = wire_client::DEFAULT_DAEMON_ADDR)]
    pub daemon: String,
}

#[derive(Args, Debug)]
pub struct TailscaleArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
    /// Daemon address (default: 127.0.0.1:9400).
    #[arg(long, default_value = wire_client::DEFAULT_DAEMON_ADDR)]
    pub daemon: String,
}

pub fn dispatch(sub: &crate::NetworkCommand, _workspace: &std::path::Path) -> Result<()> {
    match sub {
        crate::NetworkCommand::Status(a) => network_status(a),
        crate::NetworkCommand::Stun(a) => stun(a),
        crate::NetworkCommand::Tailscale(a) => tailscale(a),
    }
}

fn network_status(args: &NetworkStatusArgs) -> Result<()> {
    let msg = serde_json::json!({"type": "network_status_request"});

    match wire_client::send_message(&args.daemon, &msg) {
        Ok(response) => {
            let format = output::resolve_format(args.json, false);
            output::emit(format, &response, || {
                let upnp = response
                    .get("upnp")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                let stun = response
                    .get("stun")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                let tailscale = response
                    .get("tailscale")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();

                let upnp_status = upnp
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let stun_status = stun
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let ts_status = tailscale
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let fmt_status = |s: &str| match s {
                    "ok" | "available" | "connected" => {
                        console::style(s).green().bold().to_string()
                    }
                    "degraded" | "unavailable" => console::style(s).yellow().bold().to_string(),
                    "error" | "disabled" | "not_running" => {
                        console::style(s).red().bold().to_string()
                    }
                    _ => console::style(s).to_string(),
                };

                let mut out = format!("{}\n\n", console::style("Network status:").cyan().bold(),);
                out.push_str(&format!("  {:<16} {}\n", "UPnP:", fmt_status(upnp_status),));
                out.push_str(&format!("  {:<16} {}\n", "STUN:", fmt_status(stun_status),));
                out.push_str(&format!(
                    "  {:<16} {}\n",
                    "Tailscale:",
                    fmt_status(ts_status),
                ));

                // Show external address if available.
                if let Some(ext) = stun.get("external_addr").and_then(|v| v.as_str()) {
                    out.push_str(&format!(
                        "\n  {:<16} {}\n",
                        console::style("External IP:").cyan().bold(),
                        ext,
                    ));
                }

                // Show Tailscale peer count if available.
                if let Some(peers) = tailscale.get("peer_count").and_then(|v| v.as_u64()) {
                    out.push_str(&format!(
                        "  {:<16} {}\n",
                        console::style("TS Peers:").cyan().bold(),
                        peers,
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
                    })
                );
            } else {
                eprintln!(
                    "{} daemon not reachable at {}",
                    console::style("error:").red().bold(),
                    addr,
                );
                eprintln!("  start fabric-daemon to query network status");
            }
        }
        Err(e) => return Err(e).context("network status failed"),
    }

    Ok(())
}

fn stun(args: &StunArgs) -> Result<()> {
    let msg = serde_json::json!({"type": "network_stun_request"});

    match wire_client::send_message(&args.daemon, &msg) {
        Ok(response) => {
            let format = output::resolve_format(args.json, false);
            output::emit(format, &response, || {
                let external_addr = response
                    .get("external_addr")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(unknown)");
                let port = response
                    .get("external_port")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let mapped = response
                    .get("mapped_addr")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(unknown)");

                format!(
                    "{}\n\n\
                     {:<18} {}\n\
                     {:<18} {}\n\
                     {:<18} {}\n",
                    console::style("STUN query:").cyan().bold(),
                    console::style("External addr:").cyan().bold(),
                    external_addr,
                    console::style("External port:").cyan().bold(),
                    port,
                    console::style("Mapped addr:").cyan().bold(),
                    mapped,
                )
            })?;
        }
        Err(wire_client::WireClientError::ConnectionRefused { addr }) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "error": "daemon_unreachable",
                        "address": addr,
                    })
                );
            } else {
                eprintln!(
                    "{} daemon not reachable at {}",
                    console::style("error:").red().bold(),
                    addr,
                );
                eprintln!("  start fabric-daemon to query STUN");
            }
        }
        Err(e) => return Err(e).context("STUN query failed"),
    }

    Ok(())
}

fn tailscale(args: &TailscaleArgs) -> Result<()> {
    let msg = serde_json::json!({"type": "network_tailscale_request"});

    match wire_client::send_message(&args.daemon, &msg) {
        Ok(response) => {
            let format = output::resolve_format(args.json, false);
            output::emit(format, &response, || {
                let status = response
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let hostname = response
                    .get("hostname")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let tailnet = response
                    .get("tailnet")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let peers = response
                    .get("peers")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();

                let status_style = match status {
                    "connected" => console::style(status).green().bold(),
                    "starting" | "needs-login" => console::style(status).yellow().bold(),
                    _ => console::style(status).red().bold(),
                };

                let mut out = format!(
                    "{}\n\n\
                     {:<18} {}\n\
                     {:<18} {}\n\
                     {:<18} {}\n",
                    console::style("Tailscale status:").cyan().bold(),
                    console::style("Status:").cyan().bold(),
                    status_style,
                    console::style("Hostname:").cyan().bold(),
                    hostname,
                    console::style("Tailnet:").cyan().bold(),
                    tailnet,
                );

                if peers.is_empty() {
                    out.push_str(&format!("\n  {}\n", console::style("(no peers)").dim(),));
                } else {
                    out.push_str(&format!(
                        "\n{:<20} {:<16} {:<20} {}\n",
                        "HOSTNAME", "TAILSCALE_IP", "OS", "ONLINE"
                    ));
                    for peer in &peers {
                        let peer_host =
                            peer.get("hostname").and_then(|v| v.as_str()).unwrap_or("?");
                        let peer_ip = peer
                            .get("tailscale_ip")
                            .and_then(|v| v.as_str())
                            .unwrap_or("-");
                        let peer_os = peer.get("os").and_then(|v| v.as_str()).unwrap_or("-");
                        let online = peer
                            .get("online")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let online_label = if online {
                            console::style("yes").green().to_string()
                        } else {
                            console::style("no").red().to_string()
                        };
                        out.push_str(&format!(
                            "{:<20} {:<16} {:<20} {}\n",
                            peer_host, peer_ip, peer_os, online_label
                        ));
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
                    })
                );
            } else {
                eprintln!(
                    "{} daemon not reachable at {}",
                    console::style("error:").red().bold(),
                    addr,
                );
                eprintln!("  start fabric-daemon to query Tailscale peers");
            }
        }
        Err(e) => return Err(e).context("tailscale query failed"),
    }

    Ok(())
}
