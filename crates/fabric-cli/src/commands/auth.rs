//! `fabric auth` subcommand.
//!
//! Manage WorkOS authentication — login, check status, and logout.

use anyhow::{Context, Result};
use clap::Args;

use crate::output;
use crate::wire_client;

#[derive(Args, Debug)]
pub struct LoginArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
    /// Daemon address (default: 127.0.0.1:9400).
    #[arg(long, default_value = wire_client::DEFAULT_DAEMON_ADDR)]
    pub daemon: String,
}

#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
    /// Daemon address (default: 127.0.0.1:9400).
    #[arg(long, default_value = wire_client::DEFAULT_DAEMON_ADDR)]
    pub daemon: String,
}

#[derive(Args, Debug)]
pub struct LogoutArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

pub fn dispatch(sub: &crate::AuthCommand, _workspace: &std::path::Path) -> Result<()> {
    match sub {
        crate::AuthCommand::Login(a) => login(a),
        crate::AuthCommand::Status(a) => status(a),
        crate::AuthCommand::Logout(a) => logout(a),
    }
}

fn login(args: &LoginArgs) -> Result<()> {
    let msg = serde_json::json!({"type": "auth_login_request"});

    match wire_client::send_message(&args.daemon, &msg) {
        Ok(response) => {
            let format = output::resolve_format(args.json, false);
            output::emit(format, &response, || {
                let auth_url = response
                    .get("auth_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no url)");
                let state = response
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                format!(
                    "{} {}\n\
                     {:<18} {}\n",
                    console::style("Auth login initiated:").cyan().bold(),
                    console::style(state).green(),
                    console::style("Open URL:").cyan().bold(),
                    auth_url,
                )
            })?;

            // Print the auth URL for the user to open.
            let auth_url = response
                .get("auth_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !auth_url.is_empty() && !args.json {
                eprintln!(
                    "{} open this URL to complete login:\n  {}",
                    console::style("hint:").dim(),
                    auth_url,
                );
            }
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
                eprintln!("  start fabric-daemon to enable auth login");
            }
        }
        Err(e) => return Err(e).context("auth login failed"),
    }

    Ok(())
}

fn status(args: &StatusArgs) -> Result<()> {
    let msg = serde_json::json!({"type": "auth_status_request"});

    match wire_client::send_message(&args.daemon, &msg) {
        Ok(response) => {
            let format = output::resolve_format(args.json, false);
            output::emit(format, &response, || {
                let authenticated = response
                    .get("authenticated")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let provider = response
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or("none");
                let email = response
                    .get("email")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let expires = response
                    .get("expires_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");

                let auth_label = if authenticated {
                    console::style("authenticated").green().bold()
                } else {
                    console::style("not authenticated").red().bold()
                };

                format!(
                    "{:<18} {}\n\
                     {:<18} {}\n\
                     {:<18} {}\n\
                     {:<18} {}\n",
                    console::style("Auth status:").cyan().bold(),
                    auth_label,
                    console::style("Provider:").cyan().bold(),
                    provider,
                    console::style("Email:").cyan().bold(),
                    email,
                    console::style("Expires:").cyan().bold(),
                    expires,
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
                eprintln!("  start fabric-daemon to check auth status");
            }
        }
        Err(e) => return Err(e).context("auth status failed"),
    }

    Ok(())
}

fn logout(args: &LogoutArgs) -> Result<()> {
    // Clear the stored token file from the workspace.
    let token_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".fabric")
        .join("auth");
    let token_path = token_dir.join("token.json");

    if token_path.exists() {
        std::fs::remove_file(&token_path)
            .with_context(|| format!("remove {}", token_path.display()))?;
        if args.json {
            println!("{}", serde_json::json!({"status": "logged_out"}));
        } else {
            println!(
                "{} cleared stored credentials",
                console::style("ok").green().bold(),
            );
        }
    } else {
        if args.json {
            println!("{}", serde_json::json!({"status": "no_credentials"}));
        } else {
            println!("no stored credentials to clear");
        }
    }

    Ok(())
}
