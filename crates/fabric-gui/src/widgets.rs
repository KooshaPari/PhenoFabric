//! Liquid glass widget components for the Fabric GUI.
//!
//! All widgets accept a [`LiquidTheme`] reference so colors stay consistent.

use egui::{Color32, Pos2, Rect, Rounding, Stroke, Vec2};

use crate::animation::AnimationState;
use crate::theme::LiquidTheme;

/// A frosted glass card with gradient background, inner glow, and soft shadow.
pub fn glass_card(
    ui: &mut egui::Ui,
    title: &str,
    theme: &LiquidTheme,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let frame = LiquidTheme::glass_frame(16.0);
    frame.show(ui, |ui| {
        // Inner glow stroke
        let rect = ui.max_rect;
        let painter = ui.painter();
        painter.rect_stroke(
            rect,
            Rounding::same(16.0),
            Stroke::new(1.0, theme.glass_border_inner),
        );
        if !title.is_empty() {
            ui.label(
                egui::RichText::new(title)
                    .small()
                    .color(theme.text_secondary),
            );
            ui.add_space(4.0);
        }
        add_contents(ui);
    });
}

/// Larger glass surface for sections.
pub fn glass_panel(
    ui: &mut egui::Ui,
    theme: &LiquidTheme,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let frame = LiquidTheme::glass_frame(12.0);
    frame.show(ui, |ui| {
        add_contents(ui);
    });
}

/// Morphic stat card: big glowing number, glass background, accent bar on left.
pub fn stat_card_morphic(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    accent: Color32,
    theme: &LiquidTheme,
    anim: &AnimationState,
) {
    let frame = LiquidTheme::glass_frame(16.0);
    let response = frame.show(ui, |ui| {
        ui.set_min_width(120.0);
        // Label
        ui.label(
            egui::RichText::new(label)
                .small()
                .color(theme.text_secondary),
        );
        ui.add_space(2.0);
        // Big number with glow
        let glow_alpha = (anim.pulse_glow() * 60.0 + 20.0) as u8;
        let glow_color = Color32::from_rgba_premultiplied(
            accent.r(),
            accent.g(),
            accent.b(),
            glow_alpha,
        );
        ui.painter().rect_filled(
            ui.max_rect,
            Rounding::same(0.0),
            glow_color.linear_multiply(0.1),
        );
        ui.label(
            egui::RichText::new(value)
                .strong()
                .size(26.0)
                .color(theme.text_primary),
        );
    });
    // Accent bar on left edge
    let rect = response.response.rect;
    let painter = ui.painter();
    let bar = Rect::from_min_size(
        rect.min,
        Vec2::new(3.0, rect.height()),
    );
    painter.rect_filled(bar, Rounding::same(1.5), accent);
}

/// Morphic rounded pill with status color and glow behind it.
pub fn status_pill(
    ui: &mut egui::Ui,
    text: &str,
    color: Color32,
    theme: &LiquidTheme,
) {
    let frame = LiquidTheme::glow_frame(color);
    frame.show(ui, |ui| {
        ui.colored_label(color, text);
    });
}

/// Circular progress ring drawn with Painter arc.
pub fn progress_ring(
    ui: &mut egui::Ui,
    progress: f32,
    color: Color32,
    size: f32,
    theme: &LiquidTheme,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let painter = ui.painter();
    let center = rect.center();
    let radius = size * 0.4;
    let stroke_w = size * 0.08;

    // Background ring
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(stroke_w, theme.glass_bg_light),
    );

    // Progress arc
    let start_angle = -std::f32::consts::FRAC_PI_2;
    let sweep = progress.clamp(0.0, 1.0) * std::f32::consts::TAU;
    if sweep > 0.0 {
        let end_angle = start_angle + sweep;
        let points: Vec<Pos2> = (0..=32)
            .map(|i| {
                let t = i as f32 / 32.0;
                let angle = start_angle + (end_angle - start_angle) * t;
                Pos2::new(
                    center.x + angle.cos() * radius,
                    center.y + angle.sin() * radius,
                )
            })
            .collect();
        for pair in points.windows(2) {
            painter.line_segment([pair[0], pair[1]], Stroke::new(stroke_w, color));
        }
    }
}

/// Semicircular gauge with gradient fill.
pub fn metric_gauge(
    ui: &mut egui::Ui,
    value: f32,
    max: f32,
    color: Color32,
    size: f32,
    theme: &LiquidTheme,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(size, size * 0.6), egui::Sense::hover());
    let painter = ui.painter();
    let center = Pos2::new(rect.center().x, rect.bottom());
    let radius = size * 0.42;
    let stroke_w = size * 0.06;

    // Background arc (semicircle)
    let start = std::f32::consts::PI;
    let end = 0.0;
    let points: Vec<Pos2> = (0..=32)
        .map(|i| {
            let t = i as f32 / 32.0;
            let angle = start + (end - start) * t;
            Pos2::new(center.x + angle.cos() * radius, center.y + angle.sin() * radius)
        })
        .collect();
    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], Stroke::new(stroke_w, theme.glass_bg_light));
    }

    // Filled arc
    let ratio = (value / max.max(0.01)).clamp(0.0, 1.0);
    let fill_end = start + (end - start) * ratio;
    let fill_pts: Vec<Pos2> = (0..=32)
        .map(|i| {
            let t = i as f32 / 32.0;
            let angle = start + (fill_end - start) * t;
            Pos2::new(center.x + angle.cos() * radius, center.y + angle.sin() * radius)
        })
        .collect();
    for pair in fill_pts.windows(2) {
        painter.line_segment([pair[0], pair[1]], Stroke::new(stroke_w, color));
    }
}

/// Area chart with gradient fill under the line.
pub fn sparkline_area(
    ui: &mut egui::Ui,
    values: &[f32],
    color: Color32,
    size: Vec2,
) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    if values.len() < 2 {
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, color.linear_multiply(0.3)),
        );
        return;
    }
    let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(0.01);

    let points: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = rect.left() + (i as f32 / (values.len() - 1) as f32) * rect.width();
            let y = rect.bottom() - ((v - min) / range) * rect.height() * 0.9;
            Pos2::new(x, y)
        })
        .collect();

    let painter = ui.painter();
    // Gradient fill under line
    let mut fill_points = points.clone();
    fill_points.push(Pos2::new(rect.right(), rect.bottom()));
    fill_points.push(Pos2::new(rect.left(), rect.bottom()));
    painter.add(egui::Shape::convex_polygon(
        fill_points,
        color.linear_multiply(0.15),
        Stroke::NONE,
    ));
    // Line with glow
    for pair in points.windows(2) {
        painter.line_segment(
            [pair[0], pair[1]],
            Stroke::new(1.5, color.linear_multiply(0.3)),
        );
    }
    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], Stroke::new(1.5, color));
    }
}

/// Hoverable glass button with glow on hover.
pub fn glass_button(
    ui: &mut egui::Ui,
    label: &str,
    theme: &LiquidTheme,
) -> bool {
    let btn = egui::Button::new(
        egui::RichText::new(label).color(theme.text_primary),
    )
    .fill(theme.glass_bg_light)
    .stroke(Stroke::new(1.0, theme.glass_border))
    .rounding(Rounding::same(10.0));
    let response = ui.add(btn);
    if response.hovered() {
        let painter = ui.painter();
        painter.rect_stroke(
            response.rect,
            Rounding::same(10.0),
            Stroke::new(1.5, theme.accent_primary.linear_multiply(0.6)),
        );
    }
    response.clicked()
}

/// Frosted glass text input field.
pub fn glass_text_input(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    theme: &LiquidTheme,
) -> bool {
    let frame = egui::Frame::none()
        .fill(theme.glass_bg_light)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0, theme.glass_border))
        .inner_margin(egui::Margin::same(8.0));
    let mut changed = false;
    frame.show(ui, |ui| {
        let response = ui.add_sized(
            Vec2::new(ui.available_width(), 24.0),
            egui::TextEdit::singleline(value)
                .hint_text(egui::RichText::new(label).color(theme.text_muted))
                .frame(false),
        );
        changed = response.changed();
    });
    changed
}

/// Glowing dot for topology nodes.
pub fn node_dot(
    ui: &mut egui::Ui,
    center: Pos2,
    radius: f32,
    color: Color32,
    anim: &AnimationState,
) {
    let painter = ui.painter();
    let glow = anim.pulse_glow();
    // Outer glow halo
    painter.circle_filled(
        center,
        radius * 2.5,
        color.linear_multiply(glow * 0.15 + 0.05),
    );
    // Mid glow
    painter.circle_filled(
        center,
        radius * 1.6,
        color.linear_multiply(glow * 0.3 + 0.1),
    );
    // Core dot
    painter.circle_filled(center, radius, color);
}

/// Gradient-colored line for topology edges.
pub fn connection_line(
    ui: &mut egui::Ui,
    from: Pos2,
    to: Pos2,
    color: Color32,
) {
    let painter = ui.painter();
    // Soft shadow line
    painter.line_segment(
        [from, to],
        Stroke::new(3.0, color.linear_multiply(0.15)),
    );
    // Main line
    painter.line_segment([from, to], Stroke::new(1.5, color.linear_multiply(0.7)));
}

/// Gradient fade divider line.
pub fn section_divider(ui: &mut egui::Ui, theme: &LiquidTheme) {
    let rect = ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), 2.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {},
    )
    .response
    .rect;
    let painter = ui.painter();
    let mid_x = rect.center().x;
    let left = Pos2::new(rect.left(), rect.center().y);
    let right = Pos2::new(rect.right(), rect.center().y);
    let center = Pos2::new(mid_x, rect.center().y);
    painter.line_segment(
        [left, center],
        Stroke::new(1.0, theme.glass_border.linear_multiply(0.3)),
    );
    painter.line_segment(
        [center, right],
        Stroke::new(1.0, theme.glass_border.linear_multiply(0.3)),
    );
    // Center dot
    painter.circle_filled(center, 1.5, theme.accent_primary.linear_multiply(0.4));
}

/// Color bar that slowly shifts hues.
pub fn animated_gradient_bar(
    ui: &mut egui::Ui,
    anim: &AnimationState,
    height: f32,
    theme: &LiquidTheme,
) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    let shift = anim.gradient_shift();
    let segments = 32;
    let seg_w = rect.width() / segments as f32;
    for i in 0..segments {
        let t = i as f32 / segments as f32;
        let hue_offset = (t + shift) % 1.0;
        let color = LiquidTheme::gradient_bg(
            theme.accent_gradient_start,
            theme.accent_gradient_end,
            hue_offset,
        );
        let seg_rect = Rect::from_min_size(
            Pos2::new(rect.left() + seg_w * i as f32, rect.top()),
            Vec2::new(seg_w + 1.0, height),
        );
        painter.rect_filled(seg_rect, Rounding::ZERO, color);
    }
}

/// Styled table with alternating row colors and glass header.
pub fn styled_table<R>(
    ui: &mut egui::Ui,
    headers: &[&str],
    rows: &[R],
    theme: &LiquidTheme,
    render_row: impl Fn(&mut egui::Ui, &R, &LiquidTheme),
) {
    egui_extras::TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .columns(
            headers
                .iter()
                .map(|_| egui_extras::Column::auto().at_least(100.0))
                .collect::<Vec<_>>(),
        )
        .header(28.0, |mut header| {
            for h in headers {
                header.col(|ui| {
                    ui.label(
                        egui::RichText::new(*h)
                            .strong()
                            .small()
                            .color(theme.text_glow),
                    );
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut row| {
                let r = &rows[row.index()];
                row.col(|ui| {
                    render_row(ui, r, theme);
                });
            });
        });
}

/// Visual mini topology with glowing nodes and gradient edges.
pub fn mini_topology(
    ui: &mut egui::Ui,
    nodes: &[crate::app::TopoNode],
    edges: &[crate::app::TopoEdge],
    theme: &LiquidTheme,
    anim: &AnimationState,
    size: f32,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    if nodes.is_empty() {
        return;
    }
    let painter = ui.painter();
    let center = rect.center();
    let radius = rect.width() * 0.36;
    let positions: Vec<Pos2> = nodes
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let angle = (i as f32 / nodes.len() as f32) * std::f32::consts::TAU
                - std::f32::consts::FRAC_PI_2;
            Pos2::new(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            )
        })
        .collect();

    // Edges with glow
    for edge in edges {
        let from_idx = nodes.iter().position(|n| n.id == edge.from);
        let to_idx = nodes.iter().position(|n| n.id == edge.to);
        if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
            let color = theme.edge_color(&edge.locality);
            connection_line(ui, positions[fi], positions[ti], color);
        }
    }

    // Glowing node dots
    for (i, node) in nodes.iter().enumerate() {
        let color = theme.node_color(&node.locality);
        node_dot(ui, positions[i], 5.0, color, anim);
    }
}
