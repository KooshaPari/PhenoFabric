//! fabric-tray: System tray app for Phenotype Fabric daemon management.
//!
//! Always-running tray icon with daemon lifecycle control, health monitoring,
//! and quick-launch for TUI/GUI surfaces.

use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use fabric_tray::{check_daemon_health, create_icon, DaemonStatus};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::TrayIconBuilder;
use tao::event_loop::{ControlFlow, EventLoopBuilder};

/// User events forwarded from muda to tao event loop.
#[derive(Debug)]
enum UserEvent {
    Menu(tray_icon::menu::MenuEvent),
}

/// Application state shared across event handlers.
struct TrayApp {
    /// Path to the fabric-daemon binary.
    daemon_bin: PathBuf,
    /// Path to the fabric-cli binary.
    cli_bin: PathBuf,
    /// Config file path.
    config_path: Option<PathBuf>,
    /// The daemon child process (if spawned by us).
    daemon_process: Option<Child>,
    /// Current daemon health status.
    status: DaemonStatus,
    /// Last time we polled daemon health.
    last_poll: Instant,
    /// Daemon wire port (default 7833).
    daemon_port: u16,
}

impl TrayApp {
    fn new() -> Result<Self> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        let daemon_bin = exe_dir.join("fabric-daemon");
        let cli_bin = exe_dir.join("fabric");

        let config_path = dirs::home_dir()
            .map(|h| h.join(".fabric").join("config.toml"))
            .filter(|p| p.exists());

        Ok(Self {
            daemon_bin,
            cli_bin,
            config_path,
            daemon_process: None,
            status: DaemonStatus::Stopped,
            last_poll: Instant::now() - Duration::from_secs(10),
            daemon_port: 7833,
        })
    }

    /// Poll daemon health via TCP connection to wire server.
    fn poll_health(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_poll) < Duration::from_secs(3) {
            return;
        }
        self.last_poll = now;

        // If we have a child process, check if it's still alive
        if let Some(ref mut child) = self.daemon_process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    tracing::info!("daemon exited with status: {}", status);
                    self.daemon_process = None;
                    self.status = DaemonStatus::Stopped;
                    return;
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::error!("failed to check daemon status: {}", e);
                    self.status = DaemonStatus::Degraded;
                    return;
                }
            }
        }

        // Delegate health check to lib
        self.status = check_daemon_health(self.daemon_port);
    }

    /// Start the daemon process.
    fn start_daemon(&mut self) {
        if self.daemon_process.is_some() {
            tracing::info!("daemon already running");
            return;
        }

        let mut cmd = Command::new(&self.daemon_bin);
        cmd.arg("start");
        if let Some(ref config) = self.config_path {
            cmd.arg("--config").arg(config);
        }

        match cmd.spawn() {
            Ok(child) => {
                tracing::info!("daemon started with pid {}", child.id());
                self.daemon_process = Some(child);
                self.status = DaemonStatus::Starting;
            }
            Err(e) => {
                tracing::error!("failed to start daemon: {}", e);
                self.status = DaemonStatus::Stopped;
            }
        }
    }

    /// Stop the daemon process.
    fn stop_daemon(&mut self) {
        if let Some(ref mut child) = self.daemon_process {
            tracing::info!("stopping daemon (pid {})", child.id());
            let _ = child.kill();
            let _ = child.wait();
            self.daemon_process = None;
            self.status = DaemonStatus::Stopped;
        }
    }

    /// Open a terminal with the fabric TUI.
    fn open_tui(&self) {
        let tui_cmd = format!("{} tui", self.cli_bin.display());
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("open")
                .args(["-a", "Terminal"])
                .args(["--args", "bash", "-c", &tui_cmd])
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("x-terminal-emulator")
                .args(["-e", &tui_cmd])
                .spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("cmd")
                .args(["/C", "start", "cmd", "/K", &tui_cmd])
                .spawn();
        }
    }

    /// Open the native GUI.
    fn open_gui(&self) {
        let gui_bin = self
            .daemon_bin
            .parent()
            .map(|p| p.join("fabric-gui"))
            .unwrap_or_else(|| PathBuf::from("fabric-gui"));
        let _ = Command::new(gui_bin).spawn();
    }

    /// Open the web frontend in the default browser.
    fn open_web(&self) {
        let url = format!("http://127.0.0.1:{}", self.daemon_port);
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("open").arg(&url).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("xdg-open").arg(&url).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("cmd")
                .args(["/C", "start", &url])
                .spawn();
        }
    }

    /// Update the tray icon tooltip to reflect current status.
    fn update_tooltip(&self, tray: &tray_icon::TrayIcon) {
        let tooltip = format!("Phenotype Fabric — {}", self.status.label());
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

/// Create a menu item with id.
fn menu_item(id: &str, text: &str, enabled: bool) -> MenuItem {
    MenuItem::with_id(id, text, enabled, None)
}

/// Build the tray menu.
fn build_menu() -> Menu {
    let menu = Menu::new();

    let _ = menu.append(&menu_item("fabric-title", "Fabric — Daemon", false));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&menu_item("start-daemon", "Start Daemon", true));
    let _ = menu.append(&menu_item("stop-daemon", "Stop Daemon", true));
    let _ = menu.append(&menu_item("health-status", "Health Status", true));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&menu_item("open-tui", "Open TUI", true));
    let _ = menu.append(&menu_item("open-gui", "Open GUI", true));
    let _ = menu.append(&menu_item("open-web", "Open Web", true));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&menu_item(
        "about",
        &format!("About v{}", env!("CARGO_PKG_VERSION")),
        false,
    ));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&menu_item("quit", "Quit", true));

    menu
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("fabric-tray starting");

    let mut app = TrayApp::new()?;
    let menu = build_menu();
    let icon = create_icon(DaemonStatus::Stopped);

    // Build event loop with custom user events
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // Set up menu event forwarding to tao's event loop
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    // Create system tray icon
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("Phenotype Fabric — Stopped")
        .build()
        .context("failed to create system tray icon")?;

    tracing::info!("system tray created, entering event loop");

    let tray_icon = Some(tray_icon);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1));

        match event {
            tao::event::Event::LoopDestroyed => {
                tracing::info!("event loop shutting down");
                app.stop_daemon();
            }
            tao::event::Event::MainEventsCleared => {
                app.poll_health();
                if let Some(ref tray) = tray_icon {
                    app.update_tooltip(tray);
                }
            }
            tao::event::Event::UserEvent(UserEvent::Menu(menu_event)) => {
                let id = menu_event.id.0.as_str();
                tracing::debug!("menu event: {}", id);

                match id {
                    "start-daemon" => app.start_daemon(),
                    "stop-daemon" => app.stop_daemon(),
                    "health-status" => {
                        tracing::info!("daemon: {}", app.status.label());
                    }
                    "open-tui" => app.open_tui(),
                    "open-gui" => app.open_gui(),
                    "open-web" => app.open_web(),
                    "about" => {
                        tracing::info!(
                            "Phenotype Fabric v{}",
                            env!("CARGO_PKG_VERSION")
                        );
                    }
                    "quit" => {
                        tracing::info!("quit requested");
                        app.stop_daemon();
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });
}
