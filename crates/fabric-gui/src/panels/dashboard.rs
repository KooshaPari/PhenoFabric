//! Dashboard panel -- premium overview with morphic stat cards and health pulse.

use crate::animation::AnimationState;
use crate::app::GuiData;
use crate::premium;
use crate::theme::LiquidTheme;
use crate::widgets;

use egui::CornerRadius;

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

    // Top row: 4 morphic stat cards with sparklines
    ui.horizontal(|ui| {
        stat_card_with_sparkline(
            ui,
            "Nodes",
            &health.node_count.to_string(),
            theme.node_gpu,
            theme,
            anim,
            &generate_demo_spark(health.node_count, anim),
        );
        ui.add_space(8.0);
        stat_card_with_sparkline(
            ui,
            "Edges",
            &health.edge_count.to_string(),
            theme.accent_secondary,
            theme,
            anim,
            &generate_demo_spark(health.edge_count, anim),
        );
        ui.add_space(8.0);
        stat_card_with_sparkline(
            ui,
            "Routes",
            &health.route_count.to_string(),
            theme.accent_primary,
            theme,
            anim,
            &generate_demo_spark(health.route_count, anim),
        );
        ui.add_space(8.0);
        stat_card_with_sparkline(
            ui,
            "Leases",
            &health.lease_count.to_string(),
            theme.accent_tertiary,
            theme,
            anim,
            &generate_demo_spark(health.lease_count, anim),
        );
    });

    ui.add_space(12.0);
    widgets::section_divider(ui, theme);
    ui.add_space(8.0);

    // Middle row: health card (left) + mini topology (right)
    ui.columns(2, |cols| {
        // Left: Daemon health card with pulse ring
        widgets::glass_card(&mut cols[0], "", theme, |ui| {
            ui.set_min_width(180.0);
            premium::card_header_glass(ui, "Daemon Health", theme);

            let (dot_color, status_text) = if health.daemon_healthy {
                (theme.status_healthy, "Healthy")
            } else {
                (theme.status_error, "Offline")
            };

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                // Health pulse ring
                widgets::progress_ring(
                    ui,
                    if health.daemon_healthy { 1.0 } else { 0.0 },
                    dot_color,
                    48.0,
                    theme,
                );
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(status_text)
                            .strong()
                            .size(14.0)
                            .color(theme.text_primary),
                    );
                    ui.label(
                        egui::RichText::new(format!("Uptime: {}s", health.uptime_s))
                            .small()
                            .color(theme.text_secondary),
                    );
                    ui.label(
                        egui::RichText::new(format!("Capabilities: {}", health.cap_count))
                            .small()
                            .color(theme.text_secondary),
                    );
                });
            });
        });

        // Right: Mini topology preview
        widgets::glass_card(&mut cols[1], "", theme, |ui| {
            premium::card_header_glass(ui, "Topology Preview", theme);
            widgets::mini_topology(
                ui,
                &data.topology.nodes,
                &data.topology.edges,
                theme,
                anim,
                140.0,
            );
        });
    });

    ui.add_space(8.0);
    widgets::section_divider(ui, theme);
    ui.add_space(8.0);

    // Bottom: Activity timeline
    widgets::glass_card(ui, "", theme, |ui| {
        premium::card_header_glass(ui, "Activity", theme);
        egui::ScrollArea::vertical()
            .id_salt("dash_activity")
            .max_height(120.0)
            .show(ui, |ui| {
                if data.leases.leases.is_empty()
                    && data.routes.routes.is_empty()
                    && data.topology.nodes.is_empty()
                {
                    ui.label(
                        egui::RichText::new("No activity yet. Start the daemon to begin.")
                            .color(theme.text_muted),
                    );
                } else {
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
                    for r in data.routes.routes.iter().take(3) {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("->")
                                    .color(theme.accent_secondary),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} -> {} ({} hops)",
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

/// Stat card with an integrated sparkline area chart.
fn stat_card_with_sparkline(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    accent: egui::Color32,
    theme: &LiquidTheme,
    anim: &AnimationState,
    spark_data: &[f32],
) {
    let frame = LiquidTheme::glass_frame(16);
    let response = frame.show(ui, |ui| {
        ui.set_min_width(120.0);
        ui.label(
            egui::RichText::new(label)
                .small()
                .color(theme.text_secondary),
        );
        ui.add_space(2.0);
        // Big number with glow
        let glow_alpha = (anim.pulse_glow() * 60.0 + 20.0) as u8;
        let glow_color = egui::Color32::from_rgba_premultiplied(
            accent.r(),
            accent.g(),
            accent.b(),
            glow_alpha,
        );
        ui.painter().rect_filled(
            ui.max_rect(),
            CornerRadius::same(0),
            glow_color.linear_multiply(0.1),
        );
        ui.label(
            egui::RichText::new(value)
                .strong()
                .size(26.0)
                .color(theme.text_primary),
        );
        // Sparkline under the value
        widgets::sparkline_area(
            ui,
            spark_data,
            accent,
            egui::Vec2::new(ui.available_width(), 24.0),
        );
    });
    // Accent bar on left edge
    let rect = response.response.rect;
    let painter = ui.painter();
    let bar = egui::Rect::from_min_size(
        rect.min,
        egui::Vec2::new(3.0, rect.height()),
    );
    painter.rect_filled(bar, CornerRadius::same(2), accent);
}

/// Generate a small demo sparkline from a metric value + animation phase.
fn generate_demo_spark(base: usize, anim: &AnimationState) -> Vec<f32> {
    let n = 12;
    let base_f = base as f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32;
            let wave = (anim.gradient_shift() * 6.28 + t * 4.0).sin() * 0.3;
            base_f * (0.7 + wave + t * 0.3)
        })
        .collect()
}
