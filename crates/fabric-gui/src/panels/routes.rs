//! Routes panel — glass route cards with flow visualization.

use crate::animation::AnimationState;
use crate::app::GuiData;
use crate::theme::LiquidTheme;
use crate::widgets;

/// Render the routes view with glass cards and flow visualization.
pub fn show(ui: &mut egui::Ui, data: &GuiData, theme: &LiquidTheme, anim: &AnimationState) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Routes")
                .strong()
                .size(18.0)
                .color(theme.text_primary),
        );
        ui.separator();
        ui.label(
            egui::RichText::new(format!("{} active", data.routes.routes.len()))
                .small()
                .color(theme.text_muted),
        );
    });
    ui.add_space(6.0);
    widgets::animated_gradient_bar(ui, anim, 2.0, theme);
    ui.add_space(10.0);

    if data.routes.routes.is_empty() {
        widgets::glass_panel(ui, theme, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new("No active routes")
                        .size(16.0)
                        .color(theme.text_primary),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Compile a route plan via the daemon")
                        .color(theme.text_muted),
                );
            });
        });
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("routes_scroll")
        .show(ui, |ui| {
            for route in &data.routes.routes {
                let card_color = theme.accent_primary;
                let frame = LiquidTheme::glow_frame(card_color);
                frame.show(ui, |ui| {
                    // Route header
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&route.id)
                                .strong()
                                .color(theme.accent_secondary),
                        );
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                // Hop count badge with gradient fill
                                hop_badge(ui, route.steps, theme, anim);
                            },
                        );
                    });

                    ui.add_space(6.0);

                    // Flow visualization: source → hops → destination
                    ui.horizontal(|ui| {
                        // Source dot + label
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 5.0, theme.node_gpu);
                        ui.label(
                            egui::RichText::new(&route.source)
                                .color(theme.text_primary),
                        );

                        // Arrow hops
                        for hop_i in 0..route.steps.max(1) {
                            ui.add_space(4.0);
                            // Glowing arrow
                            let arrow_color =
                                LiquidTheme::gradient_bg(
                                    theme.accent_gradient_start,
                                    theme.accent_gradient_end,
                                    hop_i as f32 / route.steps.max(1) as f32,
                                );
                            ui.label(
                                egui::RichText::new("→")
                                    .color(arrow_color),
                            );
                            ui.add_space(2.0);
                        }

                        // Destination dot + label
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 5.0, theme.accent_tertiary);
                        ui.label(
                            egui::RichText::new(&route.destination)
                                .color(theme.text_primary),
                        );
                    });
                });
                ui.add_space(6.0);
            }
        });
}

/// Hop count badge with gradient fill.
fn hop_badge(ui: &mut egui::Ui, steps: usize, theme: &LiquidTheme, anim: &AnimationState) {
    let shift = anim.gradient_shift();
    let badge_color =
        LiquidTheme::gradient_bg(theme.accent_gradient_start, theme.accent_gradient_end, shift);
    let frame = egui::Frame::new()
        .fill(badge_color.linear_multiply(0.25))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(8, 2));
    frame.show(ui, |ui| {
        ui.label(
            egui::RichText::new(format!("{steps} hops"))
                .small()
                .strong()
                .color(badge_color),
        );
    });
}
