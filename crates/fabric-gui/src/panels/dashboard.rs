//! Dashboard panel — overview of nodes, health, routes, and leases.

use crate::app::GuiData;

const GREEN: egui::Color32 = egui::Color32::from_rgb(34, 197, 94);
const RED: egui::Color32 = egui::Color32::from_rgb(239, 68, 68);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(234, 179, 8);
const BLUE: egui::Color32 = egui::Color32::from_rgb(59, 130, 246);
const GRAY: egui::Color32 = egui::Color32::from_rgb(156, 163, 175);

/// Render the dashboard overview.
pub fn show(ui: &mut egui::Ui, data: &GuiData) {
    let health = &data.health;

    ui.horizontal(|ui| {
        ui.heading("Dashboard");
        ui.separator();
        ui.label(
            egui::RichText::new(format!("Epoch {}", health.epoch))
                .small()
                .color(GRAY),
        );
    });
    ui.separator();

    // Health cards row
    ui.horizontal(|ui| {
        health_card(ui, "Daemon", health.daemon_healthy);
        ui.separator();
        stat_card(ui, "Nodes", health.node_count);
        ui.separator();
        stat_card(ui, "Edges", health.edge_count);
        ui.separator();
        stat_card(ui, "Routes", health.route_count);
        ui.separator();
        stat_card(ui, "Leases", health.lease_count);
    });
    ui.separator();

    // Two-column layout: nodes left, routes right
    ui.columns(2, |cols| {
        // Left: Node list
        egui::ScrollArea::vertical()
            .id_salt("dash_nodes")
            .max_height(280.0)
            .show(&mut cols[0], |ui| {
                ui.strong(format!("Nodes ({})", data.topology.nodes.len()));
                if data.topology.nodes.is_empty() {
                    ui.label(egui::RichText::new("No nodes").color(GRAY));
                } else {
                    for node in &data.topology.nodes {
                        ui.horizontal(|ui| {
                            ui.colored_label(GREEN, "●");
                            let label = node
                                .label
                                .as_deref()
                                .unwrap_or(&node.id);
                            ui.label(format!("{label}  ({})", node.locality));
                        });
                    }
                }
            });

        // Right: Route summary
        egui::ScrollArea::vertical()
            .id_salt("dash_routes")
            .max_height(280.0)
            .show(&mut cols[1], |ui| {
                ui.strong(format!("Routes ({})", data.routes.routes.len()));
                if data.routes.routes.is_empty() {
                    ui.label(egui::RichText::new("No active routes").color(GRAY));
                } else {
                    for r in &data.routes.routes {
                        ui.horizontal(|ui| {
                            ui.colored_label(BLUE, "→");
                            ui.label(format!("{} → {}  ({} hops)", r.source, r.destination, r.steps));
                        });
                    }
                }
            });
    });
    ui.separator();

    // Lease summary
    ui.strong(format!("Leases ({})", data.leases.leases.len()));
    if data.leases.leases.is_empty() {
        ui.label(egui::RichText::new("No active leases").color(GRAY));
    } else {
        for l in &data.leases.leases {
            ui.horizontal(|ui| {
                let color = lease_state_color(&l.state);
                ui.colored_label(color, "●");
                ui.label(&l.name);
                ui.label(egui::RichText::new(&l.state).small().color(color));
            });
        }
    }
}

fn health_card(ui: &mut egui::Ui, label: &str, healthy: bool) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).small().color(GRAY));
        let dot_color = if healthy { GREEN } else { RED };
        let text = if healthy { "Healthy" } else { "Offline" };
        ui.horizontal(|ui| {
            ui.colored_label(dot_color, "●");
            ui.label(egui::RichText::new(text).strong());
        });
    });
}

fn stat_card(ui: &mut egui::Ui, label: &str, value: usize) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).small().color(GRAY));
        ui.label(egui::RichText::new(value.to_string()).strong().size(18.0));
    });
}

fn lease_state_color(state: &str) -> egui::Color32 {
    if state.contains("Active") {
        GREEN
    } else if state.contains("Pending") {
        YELLOW
    } else if state.contains("Failed") || state.contains("Revoked") || state.contains("Expired") {
        RED
    } else {
        GRAY
    }
}
