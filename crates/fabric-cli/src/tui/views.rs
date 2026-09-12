//! TUI view renderers for each tab.

use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Padding, Row, Table};
use ratatui::Frame;

use fabric_graph::model::TrustLevel;
use super::App;

/// Render the Topology tab — lists nodes in the topology graph.
pub fn render_topology(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Topology Nodes")
        .padding(Padding::horizontal(1));

    if let Some(ref topo) = app.data.topology {
        let header = Row::new(vec![
            Cell::from("ID"),
            Cell::from("Label"),
            Cell::from("Locality"),
            Cell::from("Caps"),
            Cell::from("Tags"),
        ])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .height(1);

        let rows: Vec<Row> = topo
            .nodes
            .values()
            .enumerate()
            .map(|(i, node)| {
                let style = if i == app.selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                let locality = format!("{:?}", node.locality_tier);
                let caps = format!("{}", node.capabilities.len());
                let tags = node.tags.join(", ");
                let label = node.label.as_deref().unwrap_or("—");

                Row::new(vec![
                    Cell::from(truncate(&node.id.0, 12)),
                    Cell::from(label),
                    Cell::from(locality_color(&locality)),
                    Cell::from(caps),
                    Cell::from(tags),
                ])
                .style(style)
            })
            .collect();

        let widths = [
            Constraint::Length(14),
            Constraint::Length(20),
            Constraint::Length(18),
            Constraint::Length(5),
            Constraint::Min(10),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_widget(table, area);
    } else {
        let empty = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No topology loaded",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Use 'fabric graph build' to create a topology",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  then press 'r' to refresh",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let para = ratatui::widgets::Paragraph::new(empty).block(block);
        f.render_widget(para, area);
    }
}

/// Render the Routes tab — lists compiled route plans.
pub fn render_routes(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Route Plans")
        .padding(Padding::horizontal(1));

    if app.data.routes.is_empty() {
        let empty = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No route plans found",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Use 'fabric route compile' to create a plan",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  then press 'r' to refresh",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let para = ratatui::widgets::Paragraph::new(empty).block(block);
        f.render_widget(para, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Intent"),
        Cell::from("Steps"),
        Cell::from("Cost"),
        Cell::from("Trust"),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .height(1);

    let rows: Vec<Row> = app
        .data
        .routes
        .iter()
        .enumerate()
        .map(|(i, route)| {
            let style = if i == app.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(truncate(&route.intent, 30)),
                Cell::from(format!("{}", route.steps)),
                Cell::from(format!("{:.1}", route.cost)),
                Cell::from(Span::styled(
                    route.trust_level.clone(),
                    trust_color(&route.trust_level),
                )),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Min(20),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(table, area);
}

/// Render the Capabilities tab — summary of capability descriptors.
pub fn render_capabilities(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Capabilities")
        .padding(Padding::horizontal(1));

    if app.data.cap_count == 0 {
        let empty = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No capability descriptors found",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Use 'fabric cap probe' to scan this machine",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  then press 'r' to refresh",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let para = ratatui::widgets::Paragraph::new(empty).block(block);
        f.render_widget(para, area);
        return;
    }

    // Show capability summary from topology nodes
    if let Some(ref topo) = app.data.topology {
        let header = Row::new(vec![
            Cell::from("Node"),
            Cell::from("Descriptor ID"),
            Cell::from("Trust Level"),
        ])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .height(1);

        let mut rows = Vec::new();
        let mut idx = 0;
        for node in topo.nodes.values() {
            for cap in &node.capabilities {
                let style = if idx == app.selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                let trust = match cap.trust {
                    TrustLevel::Untrusted => "Untrusted",
                    TrustLevel::Bootstrap => "Bootstrap",
                    TrustLevel::Attested => "Attested",
                    TrustLevel::Audited => "Audited",
                };

                rows.push(
                    Row::new(vec![
                        Cell::from(node.label.as_deref().unwrap_or(&node.id.0)),
                        Cell::from(truncate(&cap.descriptor_id, 20)),
                        Cell::from(Span::styled(trust, trust_color(trust))),
                    ])
                    .style(style),
                );
                idx += 1;
            }
        }

        let widths = [
            Constraint::Length(20),
            Constraint::Length(22),
            Constraint::Length(14),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_widget(table, area);
    } else {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {} capability descriptor(s) in workspace", app.data.cap_count),
                Style::default().fg(Color::Gray),
            )),
            Line::from(Span::styled(
                "  Load topology to view details",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let para = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(para, area);
    }
}

/// Render the Health tab — workspace and daemon status.
pub fn render_health(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Health")
        .padding(Padding::horizontal(1));

    let ws_exists = app.workspace.exists();
    let topo_exists = app.workspace.join("topology.json").exists();
    let daemon_label = if app.daemon_healthy {
        Span::styled("  ● Running", Style::default().fg(Color::Green))
    } else {
        Span::styled("  ● Offline", Style::default().fg(Color::Red))
    };

    let ws_label = if ws_exists {
        Span::styled("  ✓ Exists", Style::default().fg(Color::Green))
    } else {
        Span::styled("  ✗ Missing", Style::default().fg(Color::Red))
    };

    let topo_label = if topo_exists {
        Span::styled("  ✓ Loaded", Style::default().fg(Color::Green))
    } else {
        Span::styled("  ✗ Not found", Style::default().fg(Color::Yellow))
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Component",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::raw("  Daemon     "),
            daemon_label,
        ]),
        Line::from(vec![
            Span::raw("  Workspace  "),
            ws_label,
        ]),
        Line::from(vec![
            Span::raw("  Topology   "),
            topo_label,
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Summary",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw(format!(
            "  Nodes:        {}",
            app.data.node_count
        ))),
        Line::from(Span::raw(format!(
            "  Edges:        {}",
            app.data.edge_count
        ))),
        Line::from(Span::raw(format!(
            "  Capabilities: {}",
            app.data.cap_count
        ))),
        Line::from(Span::raw(format!(
            "  Route plans:  {}",
            app.data.routes.len()
        ))),
        Line::from(""),
    ];

    let para = ratatui::widgets::Paragraph::new(lines).block(block);
    f.render_widget(para, area);
}

// --- Helpers ---

/// Truncate a string to max_len, adding "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

/// Color for locality tier labels.
fn locality_color(tier: &str) -> Span<'static> {
    let color = match tier {
        t if t.starts_with("L0") => Color::Green,
        t if t.starts_with("L1") => Color::Green,
        t if t.starts_with("L2") => Color::Cyan,
        t if t.starts_with("L3") => Color::Cyan,
        t if t.starts_with("L4") => Color::Blue,
        t if t.starts_with("L5") => Color::Yellow,
        t if t.starts_with("L6") => Color::Yellow,
        t if t.starts_with("L7") => Color::Red,
        t if t.starts_with("L8") => Color::Magenta,
        _ => Color::White,
    };
    Span::styled(tier.to_string(), Style::default().fg(color))
}

/// Color for trust level labels.
fn trust_color(level: &str) -> Style {
    let color = match level {
        "Untrusted" => Color::Red,
        "Bootstrap" => Color::Yellow,
        "Attested" => Color::Cyan,
        "Audited" => Color::Green,
        _ => Color::White,
    };
    Style::default().fg(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        assert_eq!(truncate("hello world", 5), "hell…");
    }

    #[test]
    fn truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn locality_color_returns_cyan_for_l2() {
        let span = locality_color("L2CrossNumaShm");
        assert_eq!(span.style.fg, Some(Color::Cyan));
    }

    #[test]
    fn trust_color_untrusted_is_red() {
        let style = trust_color("Untrusted");
        assert_eq!(style.fg, Some(Color::Red));
    }

    #[test]
    fn trust_color_audited_is_green() {
        let style = trust_color("Audited");
        assert_eq!(style.fg, Some(Color::Green));
    }
}
