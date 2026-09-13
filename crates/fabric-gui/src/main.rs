//! fabric-gui: Native GUI entry point for Phenotype Fabric.
//!
//! Launches an egui desktop window with dashboard, topology, routes,
//! and leases panels. Connects to the daemon wire server or a local DB.

use eframe::egui;
use fabric_gui::app::{GuiApp, Tab};
use fabric_gui::panels;

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
        app.refresh();
        Ok(Box::new(FabricApp { state: app }))
    }))
}

struct FabricApp { state: GuiApp }

impl eframe::App for FabricApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.poll_refresh();
        if self.state.should_auto_refresh() {
            self.state.refresh();
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        self.state.handle_keys(ctx);

        egui::SidePanel::left("sidebar").default_width(160.0).show(ctx, |ui| {
            ui.heading("Fabric");
            ui.separator();
            for tab in Tab::ALL {
                let sel = self.state.active_tab == *tab;
                let txt = if sel { egui::RichText::new(format!("▸ {}", tab.title())).strong() }
                    else { egui::RichText::new(format!("  {}", tab.title())) };
                if ui.selectable_label(sel, txt).clicked() { self.state.active_tab = *tab; }
            }
            ui.separator();
            if ui.button("⟳ Refresh (R)").clicked() { self.state.refresh(); }
            let label = if self.state.dark_mode { "☀ Light" } else { "🌙 Dark" };
            if ui.button(label).clicked() { self.state.toggle_dark_mode(); self.state.configure_visuals(ctx); }
        });

        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let h = self.state.data.health.daemon_healthy;
                let c = if h { egui::Color32::from_rgb(34, 197, 94) } else { egui::Color32::from_rgb(239, 68, 68) };
                ui.colored_label(c, "●");
                ui.label(if h { "Daemon running" } else { "Daemon offline" });
                ui.separator();
                ui.label(format!("Connect: {}", self.state.connect_addr));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(ref e) = self.state.error_msg { ui.label(egui::RichText::new(e).color(egui::Color32::RED)); }
                    ui.label(format!("{}s ago", self.state.last_refresh.elapsed().as_secs()));
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state.active_tab {
                Tab::Dashboard => panels::dashboard::show(ui, &self.state.data),
                Tab::Topology => panels::topology::show(ui, &self.state.data),
                Tab::Routes => panels::routes::show(ui, &self.state.data),
                Tab::Leases => panels::leases::show(ui, &self.state.data),
            }
        });
    }
}
