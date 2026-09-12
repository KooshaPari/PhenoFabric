//! fabric-daemon: Long-running coordinator daemon for Phenotype Fabric.
//!
//! Manages topology, leases, wire transport, and health checks.
//! Persists state to SQLite via fabric-persist.

mod config;
mod coordinator;
mod health;
mod logging;
mod wire_server;

use clap::{Parser, Subcommand};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use config::DaemonConfig;
use coordinator::Coordinator;

#[derive(Parser)]
#[command(
    name = "fabric-daemon",
    version,
    about = "Long-running coordinator daemon for Phenotype Fabric"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon.
    Start {
        /// Path to config file.
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Database path (overrides config).
        #[arg(short, long)]
        db: Option<PathBuf>,

        /// Listen address (overrides config).
        #[arg(short, long)]
        listen: Option<String>,

        /// Log level (overrides config).
        #[arg(short, long)]
        log_level: Option<String>,
    },

    /// Check daemon health.
    Health {
        /// Connect to this address.
        #[arg(short, long, default_value = "127.0.0.1:9400")]
        connect: String,
    },

    /// Show daemon status.
    Status {
        /// Connect to this address.
        #[arg(short, long, default_value = "127.0.0.1:9400")]
        connect: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            config,
            db,
            listen,
            log_level,
        } => cmd_start(config, db, listen, log_level),
        Commands::Health { connect } => cmd_health(&connect),
        Commands::Status { connect } => cmd_status(&connect),
    }
}

fn cmd_start(
    config_path: Option<PathBuf>,
    db_path: Option<PathBuf>,
    listen: Option<String>,
    log_level: Option<String>,
) {
    // Load config.
    let mut config = match config_path {
        Some(path) => match DaemonConfig::from_file(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error loading config: {e}");
                std::process::exit(1);
            }
        },
        None => DaemonConfig::default(),
    };

    // Apply overrides.
    config = config.with_overrides(listen, db_path, log_level);

    // Initialize logging.
    logging::init_logging(&config.logging);

    info!("fabric-daemon starting");

    // Create coordinator.
    let coordinator = match Coordinator::new(config.clone()) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            tracing::error!("failed to create coordinator: {e}");
            std::process::exit(1);
        }
    };

    // Set up signal handling.
    let shutdown_flag = coordinator.shutdown_flag();
    let flag = shutdown_flag.clone();
    ctrlc::set_handler(move || {
        info!("received shutdown signal");
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
    })
    .expect("error setting signal handler");

    // Start wire transport server.
    let addr = coordinator.listen_addr().to_string();
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("failed to bind to {addr}: {e}");
            std::process::exit(1);
        }
    };

    let max_conn = config.server.max_connections;
    let timeout = config.server.request_timeout_ms;

    info!(addr = %addr, max_connections = max_conn, "daemon ready");

    // Run wire server (blocking until shutdown).
    if let Err(e) = wire_server::run_wire_server(listener, coordinator.clone(), max_conn, timeout)
    {
        tracing::error!("wire server error: {e}");
    }

    // Flush state before exit.
    info!("flushing state to database");
    if let Err(e) = coordinator.flush() {
        tracing::error!("flush error: {e}");
    }

    info!("fabric-daemon stopped");
}

fn cmd_health(addr: &str) {
    match std::net::TcpStream::connect(addr) {
        Ok(mut stream) => {
            use std::io::Write;
            let msg = r#"{"type":"health_check"}"#;
            let _ = write!(stream, "{msg}\n");

            use std::io::BufRead;
            let reader = std::io::BufReader::new(&stream);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        println!("{l}");
                    }
                    Err(e) => {
                        eprintln!("read error: {e}");
                        break;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("failed to connect to {addr}: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_status(addr: &str) {
    // Status uses the same health endpoint for now.
    cmd_health(addr);
}
