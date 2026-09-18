//! fabric-orchestrator: end-to-end daemon orchestrator for Phenotype Fabric.
//!
//! Wires the coordinator, persistence, wire server, and surface registry
//! into a single running process with a CLI interface.

mod pipeline;
mod serve;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use pipeline::FabricPipeline;

#[derive(Parser)]
#[command(
    name = "fabric-orchestrator",
    version,
    about = "End-to-end daemon orchestrator for Phenotype Fabric"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the full daemon (wire server + coordinator + persist + surface registry).
    Run {
        /// Path to a TOML config file.
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Listen address (overrides config).
        #[arg(short, long)]
        listen: Option<String>,

        /// Database path (overrides config).
        #[arg(short, long)]
        db: Option<PathBuf>,

        /// Log level (overrides config).
        #[arg(short, long)]
        log_level: Option<String>,
    },

    /// Load a topology JSON, compile routes, and output plans.
    Compile {
        /// Path to the topology JSON file.
        #[arg(short, long)]
        topology: PathBuf,

        /// Output file (defaults to stdout).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Run the checker against a manifest and live probe.
    Check {
        /// Path to the checker manifest JSON.
        #[arg(short, long)]
        manifest: PathBuf,

        /// Run against a live probe endpoint.
        #[arg(long)]
        probe: bool,
    },

    /// Connect to a running daemon and dump health/routes/leases.
    Status {
        /// Address of the running daemon.
        #[arg(short, long, default_value = "127.0.0.1:9400")]
        connect: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            config,
            listen,
            db,
            log_level,
        } => cmd_run(config, listen, db, log_level),
        Commands::Compile { topology, output } => cmd_compile(topology, output),
        Commands::Check { manifest, probe } => cmd_check(manifest, probe),
        Commands::Status { connect } => cmd_status(&connect),
    }
}

fn cmd_run(
    config_path: Option<PathBuf>,
    listen: Option<String>,
    db_path: Option<PathBuf>,
    log_level: Option<String>,
) -> anyhow::Result<()> {
    // Load or build config.
    let mut config = match config_path {
        Some(path) => fabric_daemon::config::DaemonConfig::from_file(&path)
            .map_err(|e| anyhow::anyhow!("config load error: {e}"))?,
        None => fabric_daemon::config::DaemonConfig::default(),
    };

    // Apply CLI overrides.
    config = config.with_overrides(listen, db_path, log_level);

    // Init logging.
    init_logging(&config.logging);

    info!("fabric-orchestrator starting");

    // Build the pipeline.
    let pipeline = FabricPipeline::new(config.clone())?;
    pipeline.start()?;

    // Set up signal handling.
    let shutdown_flag = pipeline.coordinator().shutdown_flag();
    let flag = shutdown_flag.clone();
    ctrlc::set_handler(move || {
        info!("received shutdown signal");
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
    })
    .map_err(|e| anyhow::anyhow!("signal handler error: {e}"))?;

    let addr = config.server.listen.clone();
    let max_conn = config.server.max_connections;
    let timeout = config.server.request_timeout_ms;

    // Create auth middleware from config.
    let auth_enabled = config.auth.enabled;
    let auth_config: fabric_daemon::auth::AuthMiddlewareConfig = config.auth.into();
    let auth = Arc::new(fabric_daemon::auth::AuthMiddleware::new(auth_config));
    info!(enabled = auth_enabled, "auth middleware initialized");

    // Create a dedicated tokio runtime for auth middleware async operations.
    let runtime = Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| anyhow::anyhow!("failed to create tokio runtime: {e}"))?,
    );

    info!(addr = %addr, "launching wire server");

    // Run the wire server (blocking until shutdown).
    serve::start_wire_server(
        pipeline.coordinator().clone(),
        &addr,
        max_conn,
        timeout,
        auth,
        runtime,
    )?;

    // Flush state before exit.
    info!("flushing state to database");
    pipeline.flush()?;

    info!("fabric-orchestrator stopped");
    Ok(())
}

fn cmd_compile(topology_path: PathBuf, output: Option<PathBuf>) -> anyhow::Result<()> {
    init_logging_default();

    let config = fabric_daemon::config::DaemonConfig::default();
    let pipeline = FabricPipeline::new(config)?;

    let plans = pipeline.compile_topology(&topology_path)?;

    let json = serde_json::to_string_pretty(&plans)?;

    match output {
        Some(path) => {
            std::fs::write(&path, &json)?;
            println!("wrote route plans to {}", path.display());
        }
        None => {
            println!("{json}");
        }
    }

    Ok(())
}

fn cmd_check(manifest_path: PathBuf, probe: bool) -> anyhow::Result<()> {
    init_logging_default();

    let manifest_content = std::fs::read_to_string(&manifest_path)?;
    let _manifest: fabric_checker::CheckerManifest = serde_json::from_str(&manifest_content)?;

    if probe {
        println!(
            "Running checker against manifest: {} (live probe mode)",
            manifest_path.display()
        );
        // In a full implementation, this would query the running daemon's
        // probe endpoint. For now, we report the manifest was loaded.
        println!("Manifest loaded successfully. Live probe requires a running daemon.");
    } else {
        println!("Manifest loaded: {}", manifest_path.display());
        println!("Use --probe to run against a live endpoint.");
    }

    Ok(())
}

fn cmd_status(addr: &str) -> anyhow::Result<()> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(addr)?;
    stream.set_nodelay(true)?;

    // Send health check.
    stream.write_all(b"{\"type\":\"health_check\"}")?;

    let reader = BufReader::new(&stream);
    for line in reader.lines() {
        match line {
            Ok(l) => println!("{l}"),
            Err(e) => {
                eprintln!("read error: {e}");
                break;
            }
        }
    }

    // Send routes request.
    let mut stream = TcpStream::connect(addr)?;
    stream.set_nodelay(true)?;
    stream.write_all(b"{\"type\":\"routes_request\"}")?;

    let reader = BufReader::new(&stream);
    for line in reader.lines() {
        match line {
            Ok(l) => println!("{l}"),
            Err(e) => {
                eprintln!("read error: {e}");
                break;
            }
        }
    }

    Ok(())
}

fn init_logging(logging: &fabric_daemon::config::LoggingConfig) {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&logging.level));

    match logging.format.as_str() {
        "json" => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .json()
                .init();
        }
        "compact" => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .compact()
                .init();
        }
        _ => {
            tracing_subscriber::fmt().with_env_filter(filter).init();
        }
    }
}

fn init_logging_default() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();
}
