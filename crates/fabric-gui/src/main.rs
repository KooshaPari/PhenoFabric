//! fabric-gui: Native GUI entry point for Phenotype Fabric.
//!
//! Launches an egui desktop window with topology visualization,
//! route management, and daemon health monitoring.

use std::path::PathBuf;

use eframe::egui;
use fabric_gui::{FabricGui, Panel};
use fabric_graph::model::TrustLevel;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let workspace = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".fabric")
        });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Phenotype Fabric")
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Phenotype Fabric",
        options,
        Box::new(move |cc| {
            let mut app = FabricGui::new(&workspace);
            app.load_data();
            Ok(Box::new(FabricApp::new(app, cc)))
        }),
    )
}

/// eframe application wrapper.
struct FabricApp {
    state: FabricGui,
}

impl FabricApp {
    fn new(state: FabricGui, _cc: &eframe::CreationContext<'_>) -> Self {
        Self { state }
    }
}

impl eframe::App for FabricApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Sidebar
        egui::SidePanel::left("sidebar")
            .default_width(180.0)
            .show(ctx, |ui| {
                ui.heading("Fabric");
                ui.separator();

                let panels = [
                    (Panel::Topology, "Topology", format!("{} nodes", self.state.node_count)),
                    (Panel::Routes, "Routes", format!("{} plans", self.state.routes.len())),
                    (Panel::Capabilities, "Capabilities", format!("{} descriptors", self.state.cap_count)),
                    (Panel::Health, "Health", self.state.daemon_status.label().to_string()),
                ];

                for (panel, label, detail) in panels {
                    let is_selected = self.state.panel == panel;
                    let text = if is_selected {
                        egui::RichText::new(format!("▸ {label}")).strong()
                    } else {
                        egui::RichText::new(format!("  {label}"))
                    };

                    if ui.selectable_label(is_selected, text).clicked() {
                        self.state.panel = panel;
                    }

                    ui.label(
                        egui::RichText::new(&detail)
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                }

                ui.separator();

                // Refresh button
                if ui.button("⟳ Refresh").clicked() {
                    self.state.load_data();
                }
            });

        // Status bar at top
        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let status_color = match self.state.daemon_status {
                    fabric_tray::DaemonStatus::Healthy => egui::Color32::from_rgb(34, 197, 94),
                    fabric_tray::DaemonStatus::Degraded => egui::Color32::from_rgb(239, 191, 4),
                    fabric_tray::DaemonStatus::Stopped => egui::Color32::from_rgb(239, 68, 68),
                    fabric_tray::DaemonStatus::Starting => egui::Color32::from_rgb(96, 165, 250),
                };

                ui.colored_label(status_color, "●");
                ui.label(self.state.daemon_status.label());

                ui.separator();

                ui.label(format!(
                    "Workspace: {}",
                    self.state.workspace.display()
                ));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(&self.state.status_msg);
                });
            });
        });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state.panel {
                Panel::Topology => render_topology(ui, &self.state),
                Panel::Routes => render_routes(ui, &mut self.state),
                Panel::Capabilities => render_capabilities(ui, &self.state),
                Panel::Health => render_health(ui, &self.state),
            }
        });
    }
}

// --- Tab renderers ---

fn render_topology(ui: &mut egui::Ui, state: &FabricGui) {
    ui.heading("Topology Nodes");
    ui.separator();

    if let Some(ref topo) = state.topology {
        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::auto().at_least(120.0))
            .column(egui_extras::Column::auto().at_least(150.0))
            .column(egui_extras::Column::auto().at_least(100.0))
            .column(egui_extras::Column::auto().at_least(50.0))
            .column(egui_extras::Column::remainder())
            .header(24.0, |mut header| {
                header.col(|ui| { ui.strong("ID"); });
                header.col(|ui| { ui.strong("Label"); });
                header.col(|ui| { ui.strong("Locality"); });
                header.col(|ui| { ui.strong("Caps"); });
                header.col(|ui| { ui.strong("Tags"); });
            })
            .body(|body| {
                body.rows(20.0, topo.nodes.len(), |mut row| {
                    let idx = row.index();
                    let node = topo.nodes.values().nth(idx).unwrap();
                    row.col(|ui| {
                        ui.label(&node.id.0[..node.id.0.len().min(12)]);
                    });
                    row.col(|ui| {
                        ui.label(node.label.as_deref().unwrap_or("—"));
                    });
                    row.col(|ui| {
                        ui.label(format!("{:?}", node.locality_tier));
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", node.capabilities.len()));
                    });
                    row.col(|ui| {
                        ui.label(node.tags.join(", "));
                    });
                });
            });
    } else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label("No topology loaded");
            ui.label("Use 'fabric graph build' to create a topology, then click Refresh");
        });
    }
}

fn render_routes(ui: &mut egui::Ui, state: &mut FabricGui) {
    ui.heading("Route Plans");
    ui.separator();

    if state.routes.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label("No route plans found");
            ui.label("Use 'fabric route compile' to create a plan, then click Refresh");
        });
        return;
    }

    egui_extras::TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(egui_extras::Column::auto().at_least(180.0))
        .column(egui_extras::Column::auto().at_least(60.0))
        .column(egui_extras::Column::auto().at_least(80.0))
        .column(egui_extras::Column::auto().at_least(100.0))
        .column(egui_extras::Column::remainder())
        .header(24.0, |mut header| {
            header.col(|ui| { ui.strong("Intent"); });
            header.col(|ui| { ui.strong("Steps"); });
            header.col(|ui| { ui.strong("Cost"); });
            header.col(|ui| { ui.strong("Trust"); });
            header.col(|ui| { ui.strong("File"); });
        })
        .body(|body| {
            body.rows(20.0, state.routes.len(), |mut row| {
                let idx = row.index();
                let route = &state.routes[idx];
                row.col(|ui| { ui.label(&route.intent); });
                row.col(|ui| { ui.label(format!("{}", route.steps)); });
                row.col(|ui| { ui.label(format!("{:.1}", route.cost)); });
                row.col(|ui| {
                    let color = trust_color_from_str(&route.trust_level);
                    ui.colored_label(color, &route.trust_level);
                });
                row.col(|ui| { ui.label(&route.file_name); });
            });
        });
}

fn render_capabilities(ui: &mut egui::Ui, state: &FabricGui) {
    ui.heading("Capabilities");
    ui.separator();

    if let Some(ref topo) = state.topology {
        let total_caps: usize = topo.nodes.values().map(|n| n.capabilities.len()).sum();

        if total_caps == 0 {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label("No capabilities found in topology nodes");
            });
            return;
        }

        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::auto().at_least(150.0))
            .column(egui_extras::Column::auto().at_least(180.0))
            .column(egui_extras::Column::remainder())
            .header(24.0, |mut header| {
                header.col(|ui| { ui.strong("Node"); });
                header.col(|ui| { ui.strong("Descriptor ID"); });
                header.col(|ui| { ui.strong("Trust"); });
            })
            .body(|body| {
                body.rows(20.0, total_caps, |mut row| {
                    let mut idx = row.index();
                    // Find the node and cap at this flattened index
                    let mut found = None;
                    for node in topo.nodes.values() {
                        if idx < node.capabilities.len() {
                            found = Some((node, &node.capabilities[idx]));
                            break;
                        }
                        idx -= node.capabilities.len();
                    }

                    if let Some((node, cap)) = found {
                        row.col(|ui| {
                            ui.label(node.label.as_deref().unwrap_or(&node.id.0));
                        });
                        row.col(|ui| {
                            let short = if cap.descriptor_id.len() > 20 {
                                &cap.descriptor_id[..20]
                            } else {
                                &cap.descriptor_id
                            };
                            ui.label(short);
                        });
                        row.col(|ui| {
                            let color = trust_color(cap.trust);
                            ui.colored_label(color, trust_label(cap.trust));
                        });
                    }
                });
            });
    } else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(format!("{} capability descriptor(s) in workspace", state.cap_count));
            ui.label("Load topology to view details");
        });
    }
}

fn render_health(ui: &mut egui::Ui, state: &FabricGui) {
    ui.heading("Health");
    ui.separator();

    egui::Grid::new("health_grid").num_columns(2).show(ui, |ui| {
        ui.label("Component");
        ui.label("Status");
        ui.end_row();

        ui.label("Daemon");
        let color = match state.daemon_status {
            fabric_tray::DaemonStatus::Healthy => egui::Color32::from_rgb(34, 197, 94),
            fabric_tray::DaemonStatus::Degraded => egui::Color32::from_rgb(239, 191, 4),
            fabric_tray::DaemonStatus::Stopped => egui::Color32::from_rgb(239, 68, 68),
            fabric_tray::DaemonStatus::Starting => egui::Color32::from_rgb(96, 165, 250),
        };
        ui.colored_label(color, state.daemon_status.label());
        ui.end_row();

        ui.label("Workspace");
        if state.workspace.exists() {
            ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "✓ Exists");
        } else {
            ui.colored_label(egui::Color32::from_rgb(239, 68, 68), "✗ Missing");
        }
        ui.end_row();

        ui.label("Topology");
        if state.topology.is_some() {
            ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "✓ Loaded");
        } else {
            ui.colored_label(egui::Color32::from_rgb(239, 191, 4), "✗ Not loaded");
        }
        ui.end_row();

        ui.separator();
        ui.separator();
        ui.end_row();

        ui.label("Nodes");
        ui.label(format!("{}", state.node_count));
        ui.end_row();

        ui.label("Edges");
        ui.label(format!("{}", state.edge_count));
        ui.end_row();

        ui.label("Capabilities");
        ui.label(format!("{}", state.cap_count));
        ui.end_row();

        ui.label("Route Plans");
        ui.label(format!("{}", state.routes.len()));
        ui.end_row();
    });
}

// --- Helpers ---

fn trust_color(level: TrustLevel) -> egui::Color32 {
    match level {
        TrustLevel::Untrusted => egui::Color32::from_rgb(239, 68, 68),
        TrustLevel::Bootstrap => egui::Color32::from_rgb(239, 191, 4),
        TrustLevel::Attested => egui::Color32::from_rgb(96, 165, 250),
        TrustLevel::Audited => egui::Color32::from_rgb(34, 197, 94),
    }
}

fn trust_color_from_str(s: &str) -> egui::Color32 {
    match s {
        "Untrusted" => egui::Color32::from_rgb(239, 68, 68),
        "Bootstrap" => egui::Color32::from_rgb(239, 191, 4),
        "Attested" => egui::Color32::from_rgb(96, 165, 250),
        "Audited" => egui::Color32::from_rgb(34, 197, 94),
        _ => egui::Color32::GRAY,
    }
}

fn trust_label(level: TrustLevel) -> &'static str {
    match level {
        TrustLevel::Untrusted => "Untrusted",
        TrustLevel::Bootstrap => "Bootstrap",
        TrustLevel::Attested => "Attested",
        TrustLevel::Audited => "Audited",
    }
}
