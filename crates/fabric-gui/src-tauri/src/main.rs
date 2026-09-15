//! Phenotype Fabric — Tauri v2 desktop GUI.
//!
//! Serves the liquid-glass HTML shell and communicates with fabric-daemon
//! over TCP. Daemon command handlers will be added once the wire protocol
//! is wired up; for now this is the project skeleton.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fabric_gui=info".into()),
        )
        .init();

    tracing::info!("Phenotype Fabric GUI starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("fatal: failed to run Tauri application");
}
