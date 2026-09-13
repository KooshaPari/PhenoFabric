//! Leases panel — lease table with handle, name, protocol, state (color-coded).

use crate::app::GuiData;

const GREEN: egui::Color32 = egui::Color32::from_rgb(34, 197, 94);
const RED: egui::Color32 = egui::Color32::from_rgb(239, 68, 68);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(234, 179, 8);
const GRAY: egui::Color32 = egui::Color32::from_rgb(156, 163, 175);

/// Render the leases view.
pub fn show(ui: &mut egui::Ui, data: &GuiData) {
    ui.heading("Leases");
    ui.separator();

    if data.leases.leases.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label("No active surface leases");
            ui.label(
                egui::RichText::new("Leases appear here when surfaces are bound")
                    .color(GRAY),
            );
        });
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("leases_table")
        .show(ui, |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(egui_extras::Column::auto().at_least(160.0))
                .column(egui_extras::Column::auto().at_least(140.0))
                .column(egui_extras::Column::auto().at_least(100.0))
                .column(egui_extras::Column::remainder())
                .header(22.0, |mut header| {
                    header.col(|ui| { ui.strong("Handle"); });
                    header.col(|ui| { ui.strong("Name"); });
                    header.col(|ui| { ui.strong("Protocol"); });
                    header.col(|ui| { ui.strong("State"); });
                })
                .body(|body| {
                    body.rows(20.0, data.leases.leases.len(), |mut row| {
                        let l = &data.leases.leases[row.index()];
                        let color = state_color(&l.state);
                        row.col(|ui| { ui.label(truncate_handle(&l.handle)); });
                        row.col(|ui| { ui.label(&l.name); });
                        row.col(|ui| { ui.label(&l.protocol); });
                        row.col(|ui| {
                            ui.colored_label(color, &l.state);
                        });
                    });
                });
        });
}

fn state_color(state: &str) -> egui::Color32 {
    if state.contains("Active") {
        GREEN
    } else if state.contains("Pending") {
        YELLOW
    } else if state.contains("Failed")
        || state.contains("Revoked")
        || state.contains("Expired")
    {
        RED
    } else {
        GRAY
    }
}

fn truncate_handle(h: &str) -> &str {
    if h.len() > 16 { &h[..16] } else { h }
}
