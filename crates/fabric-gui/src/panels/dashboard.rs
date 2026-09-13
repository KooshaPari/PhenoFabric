//! Dashboard panel — premium overview with morphic stat cards and health pulse.

use crate::animation::AnimationState;
use crate::app::GuiData;
use crate::theme::LiquidTheme;
use crate::widgets;

/// Render the premium dashboard overview.
pub fn show(ui: &mut egui::Ui, data: &GuiData, theme: &LiquidTheme, anim: &AnimationState) {
    let health = &data.health;

    // Header row
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Dashboard")
                .strong()
                .size(18.0)
                .color(theme.text_primary),
        );
        ui.separator();
        ui.label(
            egui::RichText::new(format!("Epoch {}", health.epoch))
                .small()
                .color(theme.text_muted),
        );
    });
    ui.add_space(8.0);

    // Animated gradient bar under header
    widgets::animated_gradient_bar(ui, anim, 2.0, theme);
    ui.add_space(10.0);

    // Top row: 4 morphic stat cards
    ui.horizontal(|ui| {
        widgets::stat_card_morphic(
            ui,
            "Nodes",
            &health.node_count.to_string(),
            theme.node_gpu,
            theme,
            anim,
        );
        ui.add_space(8.0);
        widgets::stat_card_morphic(
            ui,
            "Edges",
            &health.edge_count.to_string(),
            theme.accent_secondary,
            theme,
            anim,
        );
        ui.add_space(8.0);
        widgets::stat_card_morphic(
            ui,
            "Routes",
            &health.route_count.to_string(),
            theme.accent_primary,
            theme,
            anim,
        );
        ui.add_space(8.0);
        widgets::stat_card_morphic(
            ui,
            "Leases",
            &health.lease_count.to_string(),
            theme.accent_tertiary,
            theme,
            anim,
        );
    });

    ui.add_space(12.0);
    widgets::section_divider(ui, theme);
    ui.add_space(8.0);

    // Middle row: health card (left) + mini topology (right)
    ui.columns(2, |cols| {
        // Left: Daemon health card
        widgets::glass_card(&mut cols[0], "Daemon Health", theme, |ui| {
            ui.set_min_width(180.0);
            let (dot_color, status_text) = if health.daemon_healthy {
                (theme.status_healthy, "Healthy")
            } else {
                (theme.status_error, "Offline")
            };
            // Pulsing status indicator
            let glow = anim.pulse_glow();
            let pulse_r = 6.0 + glow * 2.0;
            let painter = ui.painter_at(ui.max_rect());
            let dot_center = ui.max_rect().left_center() + egui::vec2(12.0, 14.0);
            painter.circle_filled(
                dot_center,
                pulse_r * 2.0,
                dot_color.linear_multiply(glow * 0.2),
            );
            painter.circle_filled(dot_center, pulse_r, dot_color);

            ui.horizontal(|ui| {
                ui.add_space(28.0);
                ui.label(
                    egui::RichText::new(status_text)
                        .strong()
                        .size(14.0)
                        .color(theme.text_primary),
                );
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(28.0);
                ui.label(
                    egui::RichText::new(format!("Uptime: {}s", health.uptime_s))
                        .small()
                        .color(theme.text_secondary),
                );
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add_space(28.0);
                ui.label(
                    egui::RichText::new(format!("Capabilities: {}", health.cap_count))
                        .small()
                        .color(theme.text_secondary),
                );
            });
        });

        // Right: Mini topology preview
        widgets::glass_card(&mut cols[1], "Topology Preview", theme, |ui| {
            widgets::mini_topology(
                ui,
                &data.topology.nodes,
                &data.topology.edges,
                theme,
                anim,
                160.0,
            );
        });
    });

    ui.add_space(8.0);
    widgets::section_divider(ui, theme);
    ui.add_space(8.0);

    // Bottom: Activity timeline (last events)
    widgets::glass_card(ui, "Activity", theme, |ui| {
        egui::ScrollArea::vertical()
            .id_salt("dash_activity")
            .max_height(140.0)
            .show(ui, |ui| {
                // Show recent leases as activity
                if data.leases.leases.is_empty()
                    && data.routes.routes.is_empty()
                    && data.topology.nodes.is_empty()
                {
                    ui.label(
                        egui::RichText::new("No activity yet. Start the daemon to begin.")
                            .color(theme.text_muted),
                    );
                } else {
                    // Recent leases
                    for l in data.leases.leases.iter().take(5) {
                        let color = theme.lease_state_color(&l.state);
                        ui.horizontal(|ui| {
                            widgets::status_pill(ui, &l.state, color, theme);
                            ui.label(
                                egui::RichText::new(&l.name)
                                    .color(theme.text_primary),
                            );
                            ui.label(
                                egui::RichText::new(&l.protocol)
                                    .small()
                                    .color(theme.text_muted),
                            );
                        });
                    }
                    // Recent routes
                    for r in data.routes.routes.iter().take(3) {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("→")
                                    .color(theme.accent_secondary),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} → {}  ({} hops)",
                                    r.source, r.destination, r.steps
                                ))
                                .small()
                                .color(theme.text_secondary),
                            );
                        });
                    }
                }
            });
    });
}
