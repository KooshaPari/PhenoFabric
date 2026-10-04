//! fabric-daemon: Long-running coordinator daemon for Phenotype Fabric.
//!
//! Manages topology, leases, wire transport, and health checks.
//! Persists state to SQLite via fabric-persist.

mod auth;
mod config;
mod coordinator;
mod health;
mod logging;
mod secret_loader;
mod wire;

use clap::{Parser, Subcommand};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use config::DaemonConfig;
use coordinator::Coordinator;

use auth::{AuthMiddleware, AuthMiddlewareConfig};

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
        ///
        /// Long-only on purpose: adding `short` here would infer `-l`, which
        /// collides with `listen` above. clap's debug assertion turns that
        /// collision into a panic at startup, which meant a debug build could
        /// not start the daemon at all. Release builds were unaffected only
        /// because that assertion is compiled out under `debug_assertions`.
        /// Do not re-add a short flag that maps to `-l`.
        #[arg(long)]
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
        Some(ref path) => match DaemonConfig::from_file(path) {
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

    // Load environment secrets (e.g., WORKOS_CLIENT_SECRET).
    // Env-merge authority for deployment secrets: only nonempty env values
    // override config. The loader below re-reads WORKOS_CLIENT_SECRET once
    // solely to attribute its source (same nonempty-wins rule); keep both
    // rules in lockstep if either changes.
    config.load_env_secrets();

    // Resolve WORKOS_CLIENT_SECRET: env-first, then a nonfatal Infisical
    // fallback against the org-wide shared-secret plane (/shared/workos).
    // Prerequisites come from the merged config; only WORKOS_CLIENT_SECRET,
    // INFISICAL_ENV, and the non-secret INFISICAL_BASE_URL override are read
    // from the process env here. Runs before
    // logging init and coordinator creation; the outcome is emitted after
    // logging init below (tracing is a no-op before init) and logs
    // category/source only — never a value, body, or token.
    //
    // A rejected INFISICAL_BASE_URL fails CLOSED: the fallback is skipped
    // entirely and no request leaves the host. Substituting the documented
    // default here would POST the service-account client secret to the public
    // Infisical cloud because of a typo (review finding, 2026-09-29).
    let infisical_creds = (
        config.auth.infisical_client_id.clone(),
        config.auth.infisical_client_secret.clone(),
        config.auth.infisical_project_id.clone(),
    );
    let (secret_outcome, base_url_rejection) = secret_loader::resolve_startup_workos_secret(
        &mut config.auth,
        &|name| std::env::var(name).ok(),
        &mut |base_url, secret, folder, environment| {
            secret_loader::fetch_via_infisical(
                auth::InfisicalConfig {
                    client_id: infisical_creds.0.clone(),
                    client_secret: infisical_creds.1.clone(),
                    project_id: infisical_creds.2.clone(),
                    base_url: base_url.to_string(),
                },
                secret,
                folder,
                environment,
            )
        },
    );

    // Initialize logging.
    logging::init_logging(&config.logging);

    if let Some(rejection) = base_url_rejection {
        // Redaction-safe: category only, never the rejected value.
        tracing::warn!(
            variable = secret_loader::ENV_INFISICAL_BASE_URL,
            category = rejection.category(),
            "infisical base url override rejected; infisical fallback skipped \
             (expected https://<host>, optionally with a trailing /api)"
        );
    }
    if let Some(warning) = &secret_outcome.warning {
        tracing::warn!(
            secret = warning.secret,
            folder = warning.folder,
            environment = %warning.environment,
            category = warning.category,
            "workos client secret infisical fallback failed"
        );
    }
    info!(
        source = secret_outcome.source.as_str(),
        "workos client secret load completed"
    );

    info!("fabric-daemon starting");

    // Create coordinator.
    let coordinator = match Coordinator::new(config.clone()) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            tracing::error!("failed to create coordinator: {e}");
            std::process::exit(1);
        }
    };

    // If a config file path was provided, wire it for persistence on changes.
    if let Some(ref path) = config_path {
        coordinator.set_config_path(path.clone());
        info!(path = %path.display(), "config persistence enabled");
    }

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

    // Create auth middleware from config.
    let auth_enabled = config.auth.enabled;
    let auth_config: AuthMiddlewareConfig = config.auth.into();
    let auth = Arc::new(AuthMiddleware::new(auth_config));
    info!(enabled = auth_enabled, "auth middleware initialized");

    // Create a dedicated tokio runtime for auth middleware async operations.
    let runtime = Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime"),
    );

    // Run wire server (blocking until shutdown).
    if let Err(e) = wire::run_wire_server(
        listener,
        coordinator.clone(),
        max_conn,
        timeout,
        auth,
        runtime,
    ) {
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
            let _ = writeln!(stream, "{msg}");

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
