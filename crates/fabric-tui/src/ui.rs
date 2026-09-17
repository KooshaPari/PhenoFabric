//! All ratatui drawing functions for fabric-tui.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Tab};

// ---------------------------------------------------------------------------
// Top-level draw
// ---------------------------------------------------------------------------

pub fn draw(frame: &mut Frame, app: &App) {
    // Main layout: tabs + content + status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Min(0),    // content
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    draw_tabs(frame, app, chunks[0]);
    draw_content(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);
}

// ---------------------------------------------------------------------------
// Tab bar
// ---------------------------------------------------------------------------

fn draw_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let style = if *t == app.active_tab {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(format!(" {} [{}] ", t.title(), t.key())).style(style)
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Fabric TUI"))
        .select(
            Tab::all()
                .iter()
                .position(|t| *t == app.active_tab)
                .unwrap_or(0),
        );

    frame.render_widget(tabs, area);
}

// ---------------------------------------------------------------------------
// Content dispatcher
// ---------------------------------------------------------------------------

fn draw_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.active_tab {
        Tab::Dashboard => draw_dashboard(frame, app, area),
        Tab::Topology => draw_topology(frame, app, area),
        Tab::Routes => draw_routes(frame, app, area),
        Tab::Leases => draw_leases(frame, app, area),
    }
}

// ---------------------------------------------------------------------------
// Dashboard tab
// ---------------------------------------------------------------------------

fn draw_dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(35),
            Constraint::Percentage(30),
        ])
        .split(area);

    // Left: Topology summary
    let topo_items: Vec<ListItem> = app
        .topology
        .nodes
        .iter()
        .map(|n| {
            let label = n.label.as_deref().unwrap_or("?");
            ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("●", Style::default().fg(Color::Green)),
                Span::raw(format!(" {} ({})", label, n.locality)),
            ]))
        })
        .collect();

    let topo_list =
        List::new(topo_items).block(Block::default().borders(Borders::ALL).title(Span::styled(
            format!(" Nodes ({})", app.topology.nodes.len()),
            Style::default().fg(Color::Cyan),
        )));
    frame.render_widget(topo_list, cols[0]);

    // Center: Routes summary
    let route_items: Vec<ListItem> = app
        .routes
        .routes
        .iter()
        .map(|r| {
            ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("→", Style::default().fg(Color::Blue)),
                Span::raw(format!(
                    " {} → {} ({} hops)",
                    r.source, r.destination, r.steps
                )),
            ]))
        })
        .collect();

    let route_list = if route_items.is_empty() {
        List::new(vec![ListItem::new("  No active routes")])
    } else {
        List::new(route_items)
    }
    .block(Block::default().borders(Borders::ALL).title(Span::styled(
        format!(" Routes ({})", app.routes.routes.len()),
        Style::default().fg(Color::Blue),
    )));
    frame.render_widget(route_list, cols[1]);

    // Right: Health + Leases
    let h = &app.health;
    let health_lines = vec![
        Line::from(vec![
            Span::styled("Daemon: ", Style::default().fg(Color::Gray)),
            if h.daemon_healthy {
                Span::styled("● Healthy", Style::default().fg(Color::Green))
            } else {
                Span::styled("● Offline", Style::default().fg(Color::Red))
            },
        ]),
        Line::from(vec![
            Span::styled("Nodes: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}", h.node_count)),
        ]),
        Line::from(vec![
            Span::styled("Edges: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}", h.edge_count)),
        ]),
        Line::from(vec![
            Span::styled("Caps: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}", h.cap_count)),
        ]),
        Line::from(vec![
            Span::styled("Routes: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}", h.route_count)),
        ]),
        Line::from(vec![
            Span::styled("Leases: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}", h.lease_count)),
        ]),
        Line::from(vec![
            Span::styled("Epoch: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}", h.epoch)),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            format!("Leases ({})", app.leases.leases.len()),
            Style::default().fg(Color::Magenta),
        )),
    ];

    let mut lease_lines: Vec<Line> = app
        .leases
        .leases
        .iter()
        .map(|l| {
            let state_color = match l.state.as_str() {
                "Active" => Color::Green,
                "Pending" => Color::Yellow,
                _ => Color::Red,
            };
            Line::from(vec![
                Span::raw("  "),
                Span::styled("●", Style::default().fg(state_color)),
                Span::raw(format!(" {} [{}]", l.name, l.protocol)),
            ])
        })
        .collect();

    let mut all_lines = health_lines;
    all_lines.append(&mut lease_lines);

    let health_block = Paragraph::new(all_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(" Status ", Style::default().fg(Color::Green))),
    );
    frame.render_widget(health_block, cols[2]);
}

// ---------------------------------------------------------------------------
// Topology tab
// ---------------------------------------------------------------------------

fn draw_topology(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Label"),
        Cell::from("Locality"),
        Cell::from("Tags"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .topology
        .nodes
        .iter()
        .map(|n| {
            Row::new(vec![
                Cell::from(truncate(&n.id, 12)),
                Cell::from(n.label.as_deref().unwrap_or("-")),
                Cell::from(n.locality.as_str()),
                Cell::from(n.tags.join(", ")),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(14),
        Constraint::Length(20),
        Constraint::Length(20),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Topology — Epoch {} ", app.topology.epoch)),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = TableState::default();
    state.select(Some(
        app.selected_row
            .min(app.topology.nodes.len().saturating_sub(1)),
    ));
    frame.render_stateful_widget(table, area, &mut state);

    // Draw edges below if space
    if area.height > 10 {
        let edge_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(6),
            width: area.width,
            height: 5,
        };
        let edge_text: Vec<Line> = app
            .topology
            .edges
            .iter()
            .map(|e| {
                Line::from(vec![
                    Span::styled(&e.from, Style::default().fg(Color::Cyan)),
                    Span::raw(" ──"),
                    Span::styled(&e.locality, Style::default().fg(Color::DarkGray)),
                    Span::raw("── "),
                    Span::styled(&e.to, Style::default().fg(Color::Cyan)),
                ])
            })
            .collect();
        let edge_para = Paragraph::new(edge_text)
            .block(Block::default().borders(Borders::ALL).title(" Edges "));
        frame.render_widget(edge_para, edge_area);
    }
}

// ---------------------------------------------------------------------------
// Routes tab
// ---------------------------------------------------------------------------

fn draw_routes(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Source"),
        Cell::from("Destination"),
        Cell::from("Steps"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .routes
        .routes
        .iter()
        .map(|r| {
            Row::new(vec![
                Cell::from(truncate(&r.id, 12)),
                Cell::from(r.source.as_str()),
                Cell::from(r.destination.as_str()),
                Cell::from(format!("{}", r.steps)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(14),
        Constraint::Length(20),
        Constraint::Length(20),
        Constraint::Length(8),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Routes ({}) ", app.routes.routes.len())),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = TableState::default();
    state.select(Some(
        app.selected_row
            .min(app.routes.routes.len().saturating_sub(1)),
    ));
    frame.render_stateful_widget(table, area, &mut state);
}

// ---------------------------------------------------------------------------
// Leases tab
// ---------------------------------------------------------------------------

fn draw_leases(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Handle"),
        Cell::from("Name"),
        Cell::from("Protocol"),
        Cell::from("State"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .leases
        .leases
        .iter()
        .map(|l| {
            let state_color = match l.state.as_str() {
                "Active" => Color::Green,
                "Pending" => Color::Yellow,
                _ => Color::Red,
            };
            Row::new(vec![
                Cell::from(truncate(&l.handle, 12)),
                Cell::from(l.name.as_str()),
                Cell::from(l.protocol.as_str()),
                Cell::from(Span::styled(&l.state, Style::default().fg(state_color))),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(14),
        Constraint::Length(20),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Surface Leases ({}) ", app.leases.leases.len())),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = TableState::default();
    state.select(Some(
        app.selected_row
            .min(app.leases.leases.len().saturating_sub(1)),
    ));
    frame.render_stateful_widget(table, area, &mut state);
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mode = if app.db_path.is_some() {
        "LOCAL"
    } else {
        "LIVE"
    };
    let left = format!(" {} │ {}", mode, app.connect_addr.as_str());
    let right = if let Some(ref err) = app.error_msg {
        format!(" ⚠ {} ", err)
    } else {
        let ago = app.last_refresh.elapsed().as_secs();
        format!(" Refreshed {}s ago │ r:refresh │ q:quit │ Tab:switch ", ago)
    };

    let bar = Line::from(vec![
        Span::styled(&left, Style::default().fg(Color::White).bg(Color::DarkGray)),
        Span::raw(
            " ".repeat(area.width as usize)
                .chars()
                .take(area.width as usize - left.len() - right.len())
                .collect::<String>(),
        ),
        Span::styled(
            &right,
            Style::default().fg(Color::White).bg(Color::DarkGray),
        ),
    ]);

    let status = Paragraph::new(bar).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(status, area);
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
