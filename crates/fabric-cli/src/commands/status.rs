//! `fabric status` command.
//!
//! Connects to the daemon wire server and displays its health status.

use anyhow::{Context, Result};
use clap::Args;

use crate::output;
use crate::wire_client;

#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
    /// Daemon address (default: 127.0.0.1:9400).
    #[arg(long, default_value = wire_client::DEFAULT_DAEMON_ADDR)]
    pub daemon: String,
}

pub fn dispatch(args: &StatusArgs) -> Result<()> {
    match wire_client::health_check(&args.daemon) {
        Ok(response) => {
            let format = output::resolve_format(args.json, false);
            output::emit(format, &response, || {
                let status = response
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let uptime = response
                    .get("uptime_s")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let epoch = response
                    .get("topology_epoch")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let leases = response
                    .get("active_leases")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let plans = response
                    .get("active_plans")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let status_style = match status {
                    "healthy" => console::style(status).green().bold(),
                    "degraded" => console::style(status).yellow().bold(),
                    _ => console::style(status).red().bold(),
                };

                format!(
                    "{:<18} {}\n\
                     {:<18} {}\n\
                     {:<18} {}\n\
                     {:<18} {}\n\
                     {:<18} {} leases, {} plans\n",
                    console::style("Status:").cyan().bold(),
                    status_style,
                    console::style("Uptime:").cyan().bold(),
                    format_duration(uptime),
                    console::style("Daemon addr:").cyan().bold(),
                    args.daemon,
                    console::style("Topology epoch:").cyan().bold(),
                    epoch,
                    console::style("Active:").cyan().bold(),
                    leases,
                    plans,
                )
            })?;
        }
        Err(wire_client::WireClientError::ConnectionRefused { addr }) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "unreachable",
                        "daemon_addr": addr,
                    })
                );
            } else {
                eprintln!(
                    "{} daemon not reachable at {}",
                    console::style("error:").red().bold(),
                    addr,
                );
                eprintln!("  start fabric-daemon to enable status monitoring");
            }
            std::process::exit(1);
        }
        Err(e) => return Err(e).context("status check failed"),
    }

    Ok(())
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h {}m {}s", h, m, s)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_seconds() {
        assert_eq!(format_duration(45), "45s");
    }

    #[test]
    fn format_duration_minutes() {
        assert_eq!(format_duration(125), "2m 5s");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration(3661), "1h 1m 1s");
    }
}
