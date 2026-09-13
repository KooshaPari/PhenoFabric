//! fabric-gui: Native GUI entry point for Phenotype Fabric.
//!
//! Launches an egui desktop window with dashboard, topology, routes,
//! and leases panels. Connects to the daemon wire server or a local DB.

use eframe::egui;
use fabric_gui::app::{GuiApp, Tab};
use fabric_gui::daemon_manager::DaemonState;
use fabric_gui::panels;
use fabric_gui::panels::logs::LogViewState;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();
    let (addr, db) = GuiApp::parse_args();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Phenotype Fabric")
            .with_inner_size([1024.0, 680.0]).with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native("Phenotype Fabric", options, Box::new(move |cc| {
        let mut app = GuiApp::new(addr, db);
        app.configure_visuals(&cc.egui_ctx);
        app.auto_start_daemon();
        app.refresh();
        Ok(Box::new(FabricApp { state: app, log_state: LogViewState::default() }))
    }))
}

struct FabricApp { state: GuiApp, log_state: LogViewState }

impl eframe::App for FabricApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.poll_daemon();
        self.state.poll_refresh();
        if self.state.should_auto_refresh() {
            self.state.refresh();
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        self.state.handle_keys(ctx);

        // Sidebar
        egui::SidePanel::left("sidebar").default_width(160.0).show(ctx, |ui| {
            ui.heading("Fabric");
            ui.separator();

            // Daemon status section
            show_daemon_sidebar(ui, &mut self.state, ctx);

            ui.separator();

            // Tab navigation
            for tab in Tab::ALL {
                let sel = self.state.active_tab == *tab;
                let txt = if sel {
                    egui::RichText::new(format!("▸ {} {}", tab.icon(), tab.title())).strong()
                } else {
                    egui::RichText::new(format!("  {} {}", tab.icon(), tab.title()))
                };
                if ui.selectable_label(sel, txt).clicked() {
                    self.state.active_tab = *tab;
                }
            }
            ui.separator();
            if ui.button("⟳ Refresh (R)").clicked() {
                self.state.refresh();
            }
            let label = if self.state.dark_mode { "☀ Light" } else { "🌙 Dark" };
            if ui.button(label).clicked() {
                self.state.toggle_dark_mode();
                self.state.configure_visuals(ctx);
            }
        });

        // Status bar
        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let daemon_state = self.state.daemon_manager.state_snapshot();
                let healthy = daemon_state.is_healthy();
                let c = if healthy {
                    egui::Color32::from_rgb(34, 197, 94)
                } else {
                    egui::Color32::from_rgb(239, 68, 68)
                };
                ui.colored_label(c, "●");
                ui.label(format!("Daemon {}", daemon_state.label()));
                if let Some(pid) = daemon_state.pid() {
                    ui.label(egui::RichText::new(format!("PID {pid}")).small());
                }
                if let Some(uptime) = daemon_state.uptime_secs() {
                    ui.label(egui::RichText::new(format!("{uptime}s")).small());
                }
                let rc = self.state.daemon_manager.restart_count();
                if rc > 0 {
                    ui.label(
                        egui::RichText::new(format!("restarts: {rc}"))
                            .small()
                            .color(egui::Color32::from_rgb(234, 179, 8)),
                    );
                }
                ui.separator();
                ui.label(format!("Connect: {}", self.state.connect_addr));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(ref e) = self.state.error_msg {
                        ui.label(egui::RichText::new(e).color(egui::Color32::RED));
                    }
                    ui.label(format!("{}s ago", self.state.last_refresh.elapsed().as_secs()));
                });
            });
        });

        // Daemon log panel (bottom, toggleable)
        if self.state.show_daemon_logs {
            egui::TopBottomPanel::bottom("daemon_logs")
                .default_height(150.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong("Daemon Logs");
                        ui.separator();
                        if ui.small_button("Clear").clicked() {
                            // Logs are append-only; "clear" scrolls past them
                        }
                        if ui.small_button("✕ Close").clicked() {
                            self.state.show_daemon_logs = false;
                        }
                    });
                    egui::ScrollArea::vertical()
                        .id_salt("daemon_log_scroll")
                        .stick_to_bottom(true)
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            let logs = self.state.daemon_manager.recent_logs();
                            if logs.is_empty() {
                                ui.label(
                                    egui::RichText::new("No daemon logs yet")
                                        .italics()
                                        .color(egui::Color32::from_rgb(156, 163, 175)),
                                );
                            } else {
                                for line in logs {
                                    ui.label(
                                        egui::RichText::new(line)
                                            .monospace()
                                            .small(),
                                    );
                                }
                            }
                        });
                });
        }

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state.active_tab {
                Tab::Dashboard => panels::dashboard::show(ui, &self.state.data, &self.state.theme, &self.state.anim),
                Tab::Topology => panels::topology::show(ui, &self.state.data, &self.state.theme, &self.state.anim),
                Tab::Routes => panels::routes::show(ui, &self.state.data, &self.state.theme),
                Tab::Leases => panels::leases::show(ui, &self.state.data, &self.state.theme),
                Tab::Network => panels::network::show(ui, &self.state.data, &self.state.theme),
                Tab::Streaming => panels::streaming::show(ui, &self.state.data, &self.state.theme),
                Tab::Settings => panels::settings::show(ui, &self.state.data, &self.state.theme),
                Tab::Logs => panels::logs::show(ui, &self.state.data, &self.state.theme),
                Tab::Auth => panels::auth::show(ui, &self.state.data, &self.state.theme),
            }
        });
    }
}

/// Render the daemon section in the sidebar.
fn show_daemon_sidebar(ui: &mut egui::Ui, state: &mut GuiApp, _ctx: &egui::Context) {
    ui.label(
        egui::RichText::new("Daemon")
            .small()
            .color(egui::Color32::from_rgb(156, 163, 175)),
    );

    let daemon_state = state.daemon_manager.state_snapshot();
    let dot_color = match &daemon_state {
        DaemonState::Running { .. } => egui::Color32::from_rgb(34, 197, 94),
        DaemonState::Starting { .. } => egui::Color32::from_rgb(234, 179, 8),
        DaemonState::Failed { .. } => egui::Color32::from_rgb(239, 68, 68),
        _ => egui::Color32::from_rgb(156, 163, 175),
    };
    ui.horizontal(|ui| {
        ui.colored_label(dot_color, "●");
        ui.label(egui::RichText::new(daemon_state.label()).small());
    });

    ui.add_space(4.0);

    // Action buttons
    ui.horizontal(|ui| {
        let can_start = !matches!(
            &daemon_state,
            DaemonState::Running { .. } | DaemonState::Starting { .. }
        );
        let can_stop = !matches!(
            &daemon_state,
            DaemonState::NotStarted | DaemonState::Stopped
        );

        ui.add_enabled_ui(can_start, |ui| {
            if ui.small_button("▶ Start").clicked() {
                let _ = state.daemon_manager.start();
            }
        });
        ui.add_enabled_ui(can_stop, |ui| {
            if ui.small_button("⏹ Stop").clicked() {
                let _ = state.daemon_manager.stop();
            }
        });
        if ui.small_button("↺ Restart").clicked() {
            let _ = state.daemon_manager.restart();
        }
    });

    ui.add_space(2.0);

    // Auto-start toggle
    let mut auto = state.daemon_manager.auto_start_enabled();
    if ui.checkbox(&mut auto, "Auto-start").changed() {
        state.daemon_manager.set_auto_start(auto);
    }

    // Log toggle
    if ui.small_button("📋 Logs").clicked() {
        state.show_daemon_logs = !state.show_daemon_logs;
    }
}
