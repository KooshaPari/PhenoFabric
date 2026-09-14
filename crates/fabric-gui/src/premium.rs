//! Premium morphic widget components for the Fabric GUI.
//!
//! High-fidelity widgets built on top of the base liquid-glass theme:
//! morphic progress bars, glass icon buttons, card headers, node cards,
//! metric rows, and animated status indicators.

use egui::{Color32, Pos2, Rect, CornerRadius, Stroke, StrokeKind, Vec2};

use crate::animation::AnimationState;
use crate::theme::LiquidTheme;

/// Animated progress bar with gradient fill and outer glow.
pub fn progress_bar_morphic(
    ui: &mut egui::Ui,
    fraction: f32,
    accent: Color32,
    theme: &LiquidTheme,
    anim: &AnimationState,
) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 10.0), egui::Sense::hover());
    let painter = ui.painter();
    // Track
    painter.rect_filled(rect, CornerRadius::same(5), theme.glass_bg_light);
    // Fill
    let fill_w = rect.width() * fraction.clamp(0.0, 1.0);
    if fill_w > 1.0 {
        let fill_rect =
            Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
        let breath = anim.breathing_glow();
        // Glow halo
        let glow_rect = Rect::from_min_size(
            rect.min - Vec2::new(0.0, 2.0),
            Vec2::new(fill_w, rect.height() + 4.0),
        );
        painter.rect_filled(
            glow_rect,
            CornerRadius::same(5),
            accent.linear_multiply(breath * 0.15),
        );
        // Main fill
        painter.rect_filled(fill_rect, CornerRadius::same(5), accent);
    }
}

/// Glass icon button with hover glow effect.
/// Returns `true` when clicked.
pub fn icon_button_glass(
    ui: &mut egui::Ui,
    label: &str,
    theme: &LiquidTheme,
) -> bool {
    let btn = egui::Button::new(
        egui::RichText::new(label).size(16.0).color(theme.text_primary),
    )
    .fill(theme.glass_bg_light)
    .stroke(Stroke::new(1.0_f32, theme.glass_border))
    .corner_radius(CornerRadius::same(8));
    let response = ui.add(btn);
    if response.hovered() {
        let r = response.rect;
        ui.painter().rect_stroke(
            r,
            CornerRadius::same(8),
            Stroke::new(1.5_f32, theme.accent_primary.linear_multiply(0.7)),
            StrokeKind::Inside,
        );
    }
    response.clicked()
}

/// Card header row with gradient underline.
pub fn card_header_glass(
    ui: &mut egui::Ui,
    title: &str,
    theme: &LiquidTheme,
) {
    ui.label(
        egui::RichText::new(title)
            .strong()
            .size(13.0)
            .color(theme.text_primary),
    );
    ui.add_space(2.0);
    // Gradient underline
    let rect = ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), 2.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |_ui| {},
    )
    .response
    .rect;
    let painter = ui.painter();
    let segs = 16;
    let seg_w = rect.width() / segs as f32;
    for i in 0..segs {
        let t = i as f32 / segs as f32;
        let alpha = ((1.0 - (t - 0.5).abs() * 2.0) * 120.0) as u8;
        let color =
            Color32::from_rgba_premultiplied(108, 99, 255, alpha);
        let seg = Rect::from_min_size(
            Pos2::new(rect.left() + seg_w * i as f32, rect.top()),
            Vec2::new(seg_w + 1.0, 2.0),
        );
        painter.rect_filled(seg, CornerRadius::ZERO, color);
    }
    ui.add_space(4.0);
}

/// Topology node card with coloured indicator dot and label.
pub fn node_card(
    ui: &mut egui::Ui,
    label: &str,
    locality: &str,
    theme: &LiquidTheme,
    anim: &AnimationState,
) {
    let color = theme.node_color(locality);
    let frame = LiquidTheme::glass_frame(10);
    frame.show(ui, |ui| {
        ui.set_min_width(80.0);
        // Indicator dot with glow
        let breath = anim.breathing_glow();
        let (dot_rect, _) =
            ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
        let painter = ui.painter();
        let center = dot_rect.center();
        painter.circle_filled(
            center,
            10.0,
            color.linear_multiply(breath * 0.2),
        );
        painter.circle_filled(center, 5.0, color);
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(label)
                .small()
                .color(theme.text_primary),
        );
        ui.label(
            egui::RichText::new(locality)
                .size(10.0)
                .color(theme.text_muted),
        );
    });
}

/// Single metric row: label on the left, value on the right.
pub fn metric_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    color: Color32,
    theme: &LiquidTheme,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .small()
                .color(theme.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(value)
                    .strong()
                    .small()
                    .color(color),
            );
        });
    });
}

/// Animated status dot with pulse glow ring.
pub fn status_indicator(
    ui: &mut egui::Ui,
    color: Color32,
    anim: &AnimationState,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
    let painter = ui.painter();
    let center = rect.center();
    let glow = anim.pulse_glow();
    // Outer glow halo
    painter.circle_filled(center, 8.0, color.linear_multiply(glow * 0.25));
    // Core dot
    painter.circle_filled(center, 4.0, color);
}
