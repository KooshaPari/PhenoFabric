//! Reusable custom widget components for the Fabric GUI.
//!
//! All widgets accept a [`FabricTheme`] reference so colors stay consistent.

use egui::{Color32, Pos2, Rect, Rounding, Stroke, Vec2};

use crate::theme::FabricTheme;

/// A card with a rounded background and optional title.
///
/// Renders `add_contents` inside the card area.
pub fn card(
    ui: &mut egui::Ui,
    title: &str,
    theme: &FabricTheme,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let frame = egui::Frame::none()
        .fill(theme.bg_card)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0, theme.border_subtle))
        .inner_margin(egui::Margin::symmetric(12.0, 10.0));
    frame.show(ui, |ui| {
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

/// Stat card: large number, small label, and a colored dot.
pub fn stat_card(ui: &mut egui::Ui, label: &str, value: &str, theme: &FabricTheme) {
    let frame = egui::Frame::none()
        .fill(theme.bg_card)
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0, theme.border_subtle))
        .inner_margin(egui::Margin::symmetric(14.0, 10.0));
    frame.show(ui, |ui| {
        ui.set_min_width(100.0);
        ui.label(
            egui::RichText::new(label)
                .small()
                .color(theme.text_secondary),
        );
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.colored_label(theme.accent, "●");
            ui.label(
                egui::RichText::new(value)
                    .strong()
                    .size(22.0)
                    .color(theme.text_primary),
            );
        });
    });
}

/// Small colored pill badge with status text.
pub fn status_badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    let frame = egui::Frame::none()
        .fill(color.linear_multiply(0.2))
        .rounding(Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(6.0, 2.0));
    frame.show(ui, |ui| {
        ui.colored_label(color, text);
    });
}

/// Section header with a decorative accent line below.
pub fn section_header(ui: &mut egui::Ui, title: &str, theme: &FabricTheme) {
    ui.label(
        egui::RichText::new(title)
            .strong()
            .size(14.0)
            .color(theme.text_primary),
    );
    let rect = ui.available_rect_before_wrap();
    let painter = ui.painter();
    let y = rect.min.y + 1.0;
    painter.line_segment(
        [
            Pos2::new(rect.min.x, y),
            Pos2::new(rect.min.x + 60.0, y),
        ],
        Stroke::new(2.0, theme.accent),
    );
    ui.add_space(6.0);
}

/// Animated progress bar with rounded ends.
pub fn progress_bar(ui: &mut egui::Ui, progress: f32, color: Color32) {
    let desired_size = Vec2::new(ui.available_width(), 6.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter();
    // Track
    painter.rect_filled(rect, Rounding::same(3.0), Color32::from_gray(40));
    // Fill
    let fill_w = rect.width() * progress.clamp(0.0, 1.0);
    let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
    painter.rect_filled(fill_rect, Rounding::same(3.0), color);
}

/// Tiny inline sparkline chart.
pub fn sparkline(ui: &mut egui::Ui, values: &[f32], color: Color32, size: Vec2) {
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
    if values.len() < 2 {
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, color.linear_multiply(0.3)));
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
            let y = rect.bottom() - ((v - min) / range) * rect.height();
            Pos2::new(x, y)
        })
        .collect();

    let painter = ui.painter();
    // Fill under the line
    let mut fill_points = points.clone();
    fill_points.push(Pos2::new(rect.right(), rect.bottom()));
    fill_points.push(Pos2::new(rect.left(), rect.bottom()));
    painter.add(egui::Shape::convex_polygon(
        fill_points,
        color.linear_multiply(0.15),
        Stroke::NONE,
    ));
    // Line
    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], Stroke::new(1.5, color));
    }
}

/// Visual mini topology: colored circles for nodes, lines for edges.
pub fn mini_topology(
    ui: &mut egui::Ui,
    nodes: &[crate::app::TopoNode],
    edges: &[crate::app::TopoEdge],
    theme: &FabricTheme,
    size: f32,
) {
    let (rect, _response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    if nodes.is_empty() {
        return;
    }
    let painter = ui.painter();

    // Simple circular layout
    let center = rect.center();
    let radius = rect.width() * 0.38;
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

    // Draw edges
    for edge in edges {
        let from_idx = nodes.iter().position(|n| n.id == edge.from);
        let to_idx = nodes.iter().position(|n| n.id == edge.to);
        if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
            let color = theme.edge_color(&edge.locality).linear_multiply(0.6);
            painter.line_segment([positions[fi], positions[ti]], Stroke::new(1.0, color));
        }
    }

    // Draw nodes
    for (i, node) in nodes.iter().enumerate() {
        let color = theme.node_color(&node.locality);
        painter.circle_filled(positions[i], 5.0, color);
    }
}

/// Styled table with alternating row colors.
pub fn styled_table<R>(
    ui: &mut egui::Ui,
    headers: &[&str],
    rows: &[R],
    theme: &FabricTheme,
    render_row: impl Fn(&mut egui::Ui, &R, &FabricTheme),
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
        .header(26.0, |mut header| {
            for h in headers {
                header.col(|ui| {
                    ui.label(
                        egui::RichText::new(*h)
                            .strong()
                            .small()
                            .color(theme.text_secondary),
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
