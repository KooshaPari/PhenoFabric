//! Routes panel — route table with columns: ID, Source, Destination, Steps.

use crate::app::GuiData;

const GRAY: egui::Color32 = egui::Color32::from_rgb(156, 163, 175);
const BLUE: egui::Color32 = egui::Color32::from_rgb(59, 130, 246);

/// Render the routes view.
pub fn show(ui: &mut egui::Ui, data: &GuiData) {
    ui.heading("Routes");
    ui.separator();

    if data.routes.routes.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label("No active routes");
            ui.label(
                egui::RichText::new("Compile a route plan via the daemon")
                    .color(GRAY),
            );
        });
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("routes_table")
        .show(ui, |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(egui_extras::Column::auto().at_least(160.0))
                .column(egui_extras::Column::auto().at_least(120.0))
                .column(egui_extras::Column::auto().at_least(120.0))
                .column(egui_extras::Column::remainder())
                .header(22.0, |mut header| {
                    header.col(|ui| { ui.strong("ID"); });
                    header.col(|ui| { ui.strong("Source"); });
                    header.col(|ui| { ui.strong("Destination"); });
                    header.col(|ui| { ui.strong("Steps"); });
                })
                .body(|body| {
                    body.rows(20.0, data.routes.routes.len(), |mut row| {
                        let r = &data.routes.routes[row.index()];
                        row.col(|ui| {
                            ui.label(
                                egui::RichText::new(truncate_id(&r.id)).color(BLUE),
                            );
                        });
                        row.col(|ui| { ui.label(&r.source); });
                        row.col(|ui| { ui.label(&r.destination); });
                        row.col(|ui| { ui.label(r.steps.to_string()); });
                    });
                });
        });
}

fn truncate_id(id: &str) -> &str {
    if id.len() > 12 { &id[..12] } else { id }
}
