//! Topology panel — visual topology with morphic glowing nodes and gradient edges.

use crate::animation::AnimationState;
use crate::app::GuiData;
use crate::theme::LiquidTheme;
use crate::widgets;

/// Render the visual topology view.
pub fn show(ui: &mut egui::Ui, data: &GuiData, theme: &LiquidTheme, anim: &AnimationState) {
    let topo = &data.topology;

    // Header
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Topology")
                .strong()
                .size(18.0)
                .color(theme.text_primary),
        );
        ui.separator();
        ui.label(
            egui::RichText::new(format!(
                "Epoch {}  |  {} nodes  |  {} edges",
                topo.epoch,
                topo.nodes.len(),
                topo.edges.len(),
            ))
            .small()
            .color(theme.text_muted),
        );
    });
    ui.add_space(6.0);
    widgets::animated_gradient_bar(ui, anim, 2.0, theme);
    ui.add_space(10.0);

    if topo.nodes.is_empty() {
        widgets::glass_panel(ui, theme, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new("No topology loaded")
                        .size(16.0)
                        .color(theme.text_primary),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Start the daemon or provide a database path")
                        .color(theme.text_muted),
                );
            });
        });
        return;
    }

    // Layout: visual graph on top, node table below
    ui.columns(2, |cols| {
        // Left: Full visual topology
        widgets::glass_card(&mut cols[0], "Graph", theme, |ui| {
            let graph_size = cols[0].available_width().min(400.0);
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(graph_size, graph_size), egui::Sense::click());
            let painter = ui.painter_at(rect);

            // Subtle grid background
            let step = 20.0;
            let mut x = rect.left();
            while x <= rect.right() {
                painter.line_segment(
                    [
                        egui::pos2(x, rect.top()),
                        egui::pos2(x, rect.bottom()),
                    ],
                    egui::Stroke::new(0.5, theme.glass_border.linear_multiply(0.15)),
                );
                x += step;
            }
            let mut y = rect.top();
            while y <= rect.bottom() {
                painter.line_segment(
                    [
                        egui::pos2(rect.left(), y),
                        egui::pos2(rect.right(), y),
                    ],
                    egui::Stroke::new(0.5, theme.glass_border.linear_multiply(0.15)),
                );
                y += step;
            }

            // Compute node positions in circular layout
            let center = rect.center();
            let layout_r = rect.width() * 0.36;
            let positions: Vec<egui::Pos2> = topo
                .nodes
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let angle = (i as f32 / topo.nodes.len() as f32)
                        * std::f32::consts::TAU
                        - std::f32::consts::FRAC_PI_2;
                    egui::pos2(
                        center.x + angle.cos() * layout_r,
                        center.y + angle.sin() * layout_r,
                    )
                })
                .collect();

            // Draw edges
            for edge in &topo.edges {
                let from_idx = topo.nodes.iter().position(|n| n.id == edge.from);
                let to_idx = topo.nodes.iter().position(|n| n.id == edge.to);
                if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
                    let color = theme.edge_color(&edge.locality);
                    // Glow line
                    painter.line_segment(
                        [positions[fi], positions[ti]],
                        egui::Stroke::new(3.0, color.linear_multiply(0.15)),
                    );
                    painter.line_segment(
                        [positions[fi], positions[ti]],
                        egui::Stroke::new(1.5, color.linear_multiply(0.7)),
                    );
                }
            }

            // Draw nodes with glow
            for (i, node) in topo.nodes.iter().enumerate() {
                let color = theme.node_color(&node.locality);
                let glow = anim.pulse_glow();
                let center = positions[i];
                // Glow halo
                painter.circle_filled(
                    center,
                    16.0 + glow * 4.0,
                    color.linear_multiply(glow * 0.15 + 0.05),
                );
                painter.circle_filled(center, 10.0, color.linear_multiply(0.3));
                painter.circle_filled(center, 6.0, color);
                // Label
                let label = node.label.as_deref().unwrap_or(&node.id);
                let short_label: String = label.chars().take(8).collect();
                painter.text(
                    center + egui::vec2(0.0, 14.0),
                    egui::Align2::CENTER_TOP,
                    short_label,
                    egui::FontId::proportional(10.0),
                    theme.text_secondary,
                );
            }

            // Click detection for nodes
            if response.clicked() {
                let click_pos = response.interact_pointer_position().unwrap_or_default();
                for (i, node) in topo.nodes.iter().enumerate() {
                    if click_pos.distance(positions[i]) < 16.0 {
                        // Could show detail popup; for now highlight is sufficient
                        let _ = node;
                    }
                }
            }
        });

        // Right: Legend + node table
        widgets::glass_card(&mut cols[1], "Legend", theme, |ui| {
            ui.label(
                egui::RichText::new("Node Types").strong().color(theme.text_glow),
            );
            ui.add_space(4.0);
            legend_item(ui, "GPU", theme.node_gpu, theme);
            legend_item(ui, "CPU", theme.node_cpu, theme);
            legend_item(ui, "I/O", theme.node_io, theme);
            legend_item(ui, "NET", theme.node_network, theme);
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Edge Types").strong().color(theme.text_glow),
            );
            ui.add_space(4.0);
            legend_item(ui, "Same NUMA", theme.edge_same_numa, theme);
            legend_item(ui, "Cross NUMA", theme.edge_cross_numa, theme);
            legend_item(ui, "Cross Socket", theme.edge_cross_socket, theme);
        });
    });

    ui.add_space(8.0);

    // Node table below
    if !topo.nodes.is_empty() {
        widgets::glass_card(ui, "Nodes", theme, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("topo_nodes")
                .max_height(180.0)
                .show(ui, |ui| {
                    egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(egui_extras::Column::auto().at_least(100.0))
                        .column(egui_extras::Column::auto().at_least(120.0))
                        .column(egui_extras::Column::auto().at_least(70.0))
                        .column(egui_extras::Column::remainder())
                        .header(26.0, |mut header| {
                            header.col(|ui| {
                                ui.label(
                                    egui::RichText::new("ID").strong().small().color(theme.text_glow),
                                );
                            });
                            header.col(|ui| {
                                ui.label(
                                    egui::RichText::new("Label")
                                        .strong()
                                        .small()
                                        .color(theme.text_glow),
                                );
                            });
                            header.col(|ui| {
                                ui.label(
                                    egui::RichText::new("Locality")
                                        .strong()
                                        .small()
                                        .color(theme.text_glow),
                                );
                            });
                            header.col(|ui| {
                                ui.label(
                                    egui::RichText::new("Tags")
                                        .strong()
                                        .small()
                                        .color(theme.text_glow),
                                );
                            });
                        })
                        .body(|body| {
                            body.rows(22.0, topo.nodes.len(), |mut row| {
                                let node = &topo.nodes[row.index()];
                                row.col(|ui| {
                                    ui.label(
                                        egui::RichText::new(truncate(&node.id, 16))
                                            .color(theme.text_primary),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(
                                        egui::RichText::new(
                                            node.label.as_deref().unwrap_or("—"),
                                        )
                                        .color(theme.text_primary),
                                    );
                                });
                                row.col(|ui| {
                                    let color = theme.node_color(&node.locality);
                                    ui.colored_label(color, &node.locality);
                                });
                                row.col(|ui| {
                                    ui.label(
                                        egui::RichText::new(node.tags.join(", "))
                                            .small()
                                            .color(theme.text_secondary),
                                    );
                                });
                            });
                        });
                });
        });
    }
}

fn legend_item(ui: &mut egui::Ui, label: &str, color: egui::Color32, theme: &LiquidTheme) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, color);
        ui.label(
            egui::RichText::new(label)
                .small()
                .color(theme.text_secondary),
        );
    });
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() > max {
        &s[..max]
    } else {
        s
    }
}
