//! Topology panel — node table with columns, edge visualization, epoch display.

use crate::app::GuiData;

const GRAY: egui::Color32 = egui::Color32::from_rgb(156, 163, 175);

/// Render the topology view.
pub fn show(ui: &mut egui::Ui, data: &GuiData) {
    let topo = &data.topology;

    ui.horizontal(|ui| {
        ui.heading("Topology");
        ui.separator();
        ui.label(
            egui::RichText::new(format!(
                "Epoch {}  |  {} nodes  |  {} edges",
                topo.epoch,
                topo.nodes.len(),
                topo.edges.len(),
            ))
            .small()
            .color(GRAY),
        );
    });
    ui.separator();

    if topo.nodes.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label("No topology loaded");
            ui.label(
                egui::RichText::new("Start the daemon or provide a database path")
                    .color(GRAY),
            );
        });
        return;
    }

    // Node table
    ui.strong("Nodes");
    egui::ScrollArea::vertical()
        .id_salt("topo_nodes")
        .show(ui, |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(egui_extras::Column::auto().at_least(120.0))
                .column(egui_extras::Column::auto().at_least(140.0))
                .column(egui_extras::Column::auto().at_least(80.0))
                .column(egui_extras::Column::remainder())
                .header(22.0, |mut header| {
                    header.col(|ui| { ui.strong("ID"); });
                    header.col(|ui| { ui.strong("Label"); });
                    header.col(|ui| { ui.strong("Locality"); });
                    header.col(|ui| { ui.strong("Tags"); });
                })
                .body(|body| {
                    body.rows(20.0, topo.nodes.len(), |mut row| {
                        let node = &topo.nodes[row.index()];
                        row.col(|ui| { ui.label(truncate(&node.id, 16)); });
                        row.col(|ui| {
                            ui.label(node.label.as_deref().unwrap_or("—"));
                        });
                        row.col(|ui| { ui.label(&node.locality); });
                        row.col(|ui| { ui.label(node.tags.join(", ")); });
                    });
                });
        });

    ui.add_space(8.0);

    // Edge summary
    if !topo.edges.is_empty() {
        ui.strong("Edges");
        egui::ScrollArea::vertical()
            .id_salt("topo_edges")
            .max_height(160.0)
            .show(ui, |ui| {
                egui::Grid::new("edge_grid").striped(true).show(ui, |ui| {
                    ui.strong("From");
                    ui.strong("To");
                    ui.strong("Locality");
                    ui.end_row();
                    for edge in &topo.edges {
                        ui.label(truncate(&edge.from, 16));
                        ui.label(truncate(&edge.to, 16));
                        ui.label(&edge.locality);
                        ui.end_row();
                    }
                });
            });
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() > max { &s[..max] } else { s }
}
