//! Leases panel — morphic lease cards with protocol badges and status glow.

use crate::animation::AnimationState;
use crate::app::GuiData;
use crate::theme::LiquidTheme;
use crate::widgets;

/// Render the leases view with glass cards and morphic badges.
pub fn show(ui: &mut egui::Ui, data: &GuiData, theme: &LiquidTheme, anim: &AnimationState) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Leases")
                .strong()
                .size(18.0)
                .color(theme.text_primary),
        );
        ui.separator();
        ui.label(
            egui::RichText::new(format!("{} active", data.leases.leases.len()))
                .small()
                .color(theme.text_muted),
        );
    });
    ui.add_space(6.0);
    widgets::animated_gradient_bar(ui, anim, 2.0, theme);
    ui.add_space(10.0);

    if data.leases.leases.is_empty() {
        widgets::glass_panel(ui, theme, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new("No active surface leases")
                        .size(16.0)
                        .color(theme.text_primary),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Leases appear here when surfaces are bound")
                        .color(theme.text_muted),
                );
            });
        });
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("leases_scroll")
        .show(ui, |ui| {
            for lease in &data.leases.leases {
                let state_color = theme.lease_state_color(&lease.state);
                let frame = LiquidTheme::glow_frame(state_color);
                frame.show(ui, |ui| {
                    // Header row: name + state pill
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&lease.name)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                // State indicator with glow
                                let glow = anim.pulse_glow();
                                let pulse_alpha = if lease.state.contains("Active") {
                                    (glow * 40.0 + 20.0) as u8
                                } else {
                                    20
                                };
                                let glow_bg = egui::Color32::from_rgba_premultiplied(
                                    state_color.r(),
                                    state_color.g(),
                                    state_color.b(),
                                    pulse_alpha,
                                );
                                let pill_frame = egui::Frame::new()
                                    .fill(glow_bg)
                                    .rounding(egui::Rounding::same(10))
                                    .inner_margin(egui::Margin::symmetric(10, 3));
                                pill_frame.show(ui, |ui| {
                                    ui.colored_label(state_color, &lease.state);
                                });
                            },
                        );
                    });

                    ui.add_space(4.0);

                    // Details row: handle + protocol badge
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(truncate_handle(&lease.handle))
                                .small()
                                .color(theme.text_secondary),
                        );
                        ui.add_space(8.0);
                        // Protocol badge with morphic styling
                        protocol_badge(ui, &lease.protocol, theme);
                    });
                });
                ui.add_space(6.0);
            }
        });
}

/// Protocol badge with morphic glass styling.
fn protocol_badge(ui: &mut egui::Ui, protocol: &str, theme: &LiquidTheme) {
    let color = match protocol.to_lowercase().as_str() {
        s if s.contains("rdma") => theme.node_gpu,
        s if s.contains("nvlink") => theme.accent_secondary,
        s if s.contains("pcie") => theme.accent_primary,
        _ => theme.node_default,
    };
    let frame = egui::Frame::new()
        .fill(color.linear_multiply(0.2))
        .rounding(egui::Rounding::same(6))
        .inner_margin(egui::Margin::symmetric(8, 2));
    frame.show(ui, |ui| {
        ui.colored_label(color, protocol);
    });
}

fn truncate_handle(h: &str) -> &str {
    if h.len() > 16 {
        &h[..16]
    } else {
        h
    }
}
