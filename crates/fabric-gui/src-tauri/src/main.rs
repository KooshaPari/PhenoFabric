//! Phenotype Fabric -- Tauri v2 desktop GUI.
//!
//! Serves the liquid-glass HTML shell and communicates with fabric-daemon
//! over TCP. Daemon command handlers bridge frontend invoke() calls to the
//! async wire-protocol client in daemon.rs.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth_callback;
mod commands;
mod daemon;
mod types;

use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared application state injected into Tauri commands via `State<>`.
pub struct AppState {
    pub daemon: Arc<Mutex<daemon::DaemonManager>>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fabric_gui=info".into()),
        )
        .init();

    tracing::info!("Phenotype Fabric GUI starting");

    let daemon_manager = daemon::DaemonManager::new(daemon::DaemonConfig::default_config());
    let app_state = AppState {
        daemon: Arc::new(Mutex::new(daemon_manager)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_health,
            commands::get_topology,
            commands::get_routes,
            commands::get_leases,
            commands::get_network,
            commands::get_streaming,
            commands::get_auth,
            commands::get_logs,
            commands::get_settings,
            commands::refresh_data,
            commands::start_daemon,
            commands::stop_daemon,
            commands::restart_daemon,
            commands::get_daemon_status,
            commands::start_auth,
            commands::complete_auth,
            commands::start_email_auth,
            commands::start_auth_listener,
        ])
        .run(tauri::generate_context!())
        .expect("fatal: failed to run Tauri application");
}
