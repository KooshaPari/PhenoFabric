//! fabric-tui: Terminal dashboard for Phenotype Fabric daemon.
//!
//! Displays topology, routes, surface leases, and health in a ratatui TUI.
//! Connects to the daemon wire server via TCP for live data, or reads from
//! a local SQLite database.

use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use clap::Parser;
use ratatui::prelude::*;
use ratatui::widgets::*;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Deserialize)]
struct TopologyResponse {
    nodes: Vec<TopoNode>,
    edges: Vec<TopoEdge>,
    epoch: u64,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct TopoNode {
    id: String,
    label: Option<String>,
    locality: String,
    tags: Vec<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct TopoEdge {
    from: String,
    to: String,
    locality: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct HealthResponse {
    daemon_healthy: bool,
    uptime_s: u64,
    node_count: usize,
    edge_count: usize,
    cap_count: usize,
    route_count: usize,
    lease_count: usize,
    epoch: u64,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct RoutesResponse {
    routes: Vec<RouteInfo>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct RouteInfo {
    id: String,
    steps: usize,
    source: String,
    destination: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct LeasesResponse {
    leases: Vec<LeaseInfo>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct LeaseInfo {
    handle: String,
    protocol: String,
    state: String,
    name: String,
}

// ---------------------------------------------------------------------------
// Tab state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Dashboard,
    Topology,
    Routes,
    Leases,
}

impl Tab {
    fn all() -> &'static [Tab] {
        &[Tab::Dashboard, Tab::Topology, Tab::Routes, Tab::Leases]
    }

    fn title(&self) -> &str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Topology => "Topology",
            Tab::Routes => "Routes",
            Tab::Leases => "Leases",
        }
    }

    fn key(&self) -> char {
        match self {
            Tab::Dashboard => '1',
            Tab::Topology => '2',
            Tab::Routes => '3',
            Tab::Leases => '4',
        }
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct App {
    running: bool,
    active_tab: Tab,
    topology: TopologyResponse,
    health: HealthResponse,
    routes: RoutesResponse,
    leases: LeasesResponse,
    selected_row: usize,
    last_refresh: Instant,
    connect_addr: String,
    error_msg: Option<String>,
    db_path: Option<String>,
}

impl App {
    fn new(connect_addr: String, db_path: Option<String>) -> Self {
        Self {
            running: true,
            active_tab: Tab::Dashboard,
            topology: TopologyResponse::default(),
            health: HealthResponse::default(),
            routes: RoutesResponse::default(),
            leases: LeasesResponse::default(),
            selected_row: 0,
            last_refresh: Instant::now() - Duration::from_secs(10), // force initial refresh
            connect_addr,
            error_msg: None,
            db_path,
        }
    }

    fn refresh(&mut self) {
        let db_path = self.db_path.clone();
        if let Some(ref path) = db_path {
            self.refresh_from_db(path);
        } else {
            self.refresh_from_daemon();
        }
        self.last_refresh = Instant::now();
    }

    fn refresh_from_daemon(&mut self) {
        self.error_msg = None;

        // Health
        if let Ok(h) = fetch_daemon_json::<HealthResponse>(&self.connect_addr, "health_check") {
            self.health = h;
        } else {
            self.error_msg = Some(format!("Cannot reach daemon at {}", self.connect_addr));
            return;
        }

        // Topology
        if let Ok(t) = fetch_daemon_json::<TopologyResponse>(&self.connect_addr, "topology_request") {
            self.topology = t;
        }

        // Routes
        if let Ok(r) = fetch_daemon_json::<RoutesResponse>(&self.connect_addr, "routes_request") {
            self.routes = r;
        }

        // Leases
        if let Ok(l) = fetch_daemon_json::<LeasesResponse>(&self.connect_addr, "leases_request") {
            self.leases = l;
        }
    }

    fn refresh_from_db(&mut self, path: &str) {
        self.error_msg = None;
        match fabric_persist::Persist::open(path) {
            Ok(persist) => {
                if let Ok(Some(topo)) = persist.load_topology() {
                    self.topology = TopologyResponse {
                        nodes: topo.nodes.values().map(|n| TopoNode {
                            id: n.id.0.clone(),
                            label: n.label.clone(),
                            locality: format!("{:?}", n.locality_tier),
                            tags: n.tags.clone(),
                        }).collect(),
                        edges: topo.edges.values().map(|e| TopoEdge {
                            from: e.from.0.clone(),
                            to: e.to.0.clone(),
                            locality: format!("{:?}", e.locality_tier),
                        }).collect(),
                        epoch: topo.epoch.0,
                    };
                    self.health.node_count = self.topology.nodes.len();
                    self.health.edge_count = self.topology.edges.len();
                    self.health.epoch = self.topology.epoch;
                }
                if let Ok(state) = persist.recover_state() {
                    self.health.lease_count = state.active_leases.len();
                    self.health.route_count = state.active_plans.len();
                    self.leases = LeasesResponse {
                        leases: state.active_leases.iter().map(|l| LeaseInfo {
                            handle: format!("{}", l.handle.0),
                            protocol: format!("{:?}", l.spec.protocol),
                            state: format!("{:?}", l.state),
                            name: l.spec.name.clone(),
                        }).collect(),
                    };
                }
            }
            Err(e) => {
                self.error_msg = Some(format!("DB error: {e}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Wire protocol helpers
// ---------------------------------------------------------------------------

fn fetch_daemon_json<T: for<'de> Deserialize<'de>>(addr: &str, msg_type: &str) -> Result<T, String> {
    let mut stream = TcpStream::connect(addr)
        .map_err(|e| format!("connect failed: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(3))).ok();

    let msg = format!("{{\"type\":\"{}\"}}\n", msg_type);
    stream.write_all(msg.as_bytes()).map_err(|e| format!("write failed: {e}"))?;

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| format!("read failed: {e}"))?;

    serde_json::from_str(&line).map_err(|e| format!("parse failed: {e}"))
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

fn draw(frame: &mut Frame, app: &App) {
    // Main layout: tabs + content + status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // tabs
            Constraint::Min(0),    // content
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    draw_tabs(frame, app, chunks[0]);
    draw_content(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);
}

fn draw_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let style = if *t == app.active_tab {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(format!(" {} [{}] ", t.title(), t.key())).style(style)
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Fabric TUI"))
        .select(Tab::all().iter().position(|t| *t == app.active_tab).unwrap_or(0));

    frame.render_widget(tabs, area);
}

fn draw_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.active_tab {
        Tab::Dashboard => draw_dashboard(frame, app, area),
        Tab::Topology => draw_topology(frame, app, area),
        Tab::Routes => draw_routes(frame, app, area),
        Tab::Leases => draw_leases(frame, app, area),
    }
}

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
    let topo_items: Vec<ListItem> = app.topology.nodes.iter().map(|n| {
        let label = n.label.as_deref().unwrap_or("?");
        ListItem::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("●", Style::default().fg(Color::Green)),
            Span::raw(format!(" {} ({})", label, n.locality)),
        ]))
    }).collect();

    let topo_list = List::new(topo_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                format!(" Nodes ({})", app.topology.nodes.len()),
                Style::default().fg(Color::Cyan),
            )));
    frame.render_widget(topo_list, cols[0]);

    // Center: Routes summary
    let route_items: Vec<ListItem> = app.routes.routes.iter().map(|r| {
        ListItem::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("→", Style::default().fg(Color::Blue)),
            Span::raw(format!(" {} → {} ({} hops)", r.source, r.destination, r.steps)),
        ]))
    }).collect();

    let route_list = if route_items.is_empty() {
        List::new(vec![ListItem::new("  No active routes")])
    } else {
        List::new(route_items)
    }.block(Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
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

    let mut lease_lines: Vec<Line> = app.leases.leases.iter().map(|l| {
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
    }).collect();

    let mut all_lines = health_lines;
    all_lines.append(&mut lease_lines);

    let health_block = Paragraph::new(all_lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                " Status ",
                Style::default().fg(Color::Green),
            )));
    frame.render_widget(health_block, cols[2]);
}

fn draw_topology(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Label"),
        Cell::from("Locality"),
        Cell::from("Tags"),
    ]).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app.topology.nodes.iter().map(|n| {
        Row::new(vec![
            Cell::from(truncate(&n.id, 12)),
            Cell::from(n.label.as_deref().unwrap_or("-")),
            Cell::from(n.locality.as_str()),
            Cell::from(n.tags.join(", ")),
        ])
    }).collect();

    let widths = [
        Constraint::Length(14),
        Constraint::Length(20),
        Constraint::Length(20),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Topology — Epoch {} ", app.topology.epoch)))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = TableState::default();
    state.select(Some(app.selected_row.min(app.topology.nodes.len().saturating_sub(1))));
    frame.render_stateful_widget(table, area, &mut state);

    // Draw edges below if space
    if area.height > 10 {
        let edge_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(6),
            width: area.width,
            height: 5,
        };
        let edge_text: Vec<Line> = app.topology.edges.iter().map(|e| {
            Line::from(vec![
                Span::styled(&e.from, Style::default().fg(Color::Cyan)),
                Span::raw(" ──"),
                Span::styled(&e.locality, Style::default().fg(Color::DarkGray)),
                Span::raw("── "),
                Span::styled(&e.to, Style::default().fg(Color::Cyan)),
            ])
        }).collect();
        let edge_para = Paragraph::new(edge_text)
            .block(Block::default().borders(Borders::ALL).title(" Edges "));
        frame.render_widget(edge_para, edge_area);
    }
}

fn draw_routes(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Source"),
        Cell::from("Destination"),
        Cell::from("Steps"),
    ]).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app.routes.routes.iter().map(|r| {
        Row::new(vec![
            Cell::from(truncate(&r.id, 12)),
            Cell::from(r.source.as_str()),
            Cell::from(r.destination.as_str()),
            Cell::from(format!("{}", r.steps)),
        ])
    }).collect();

    let widths = [
        Constraint::Length(14),
        Constraint::Length(20),
        Constraint::Length(20),
        Constraint::Length(8),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Routes ({}) ", app.routes.routes.len())))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = TableState::default();
    state.select(Some(app.selected_row.min(app.routes.routes.len().saturating_sub(1))));
    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_leases(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Handle"),
        Cell::from("Name"),
        Cell::from("Protocol"),
        Cell::from("State"),
    ]).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app.leases.leases.iter().map(|l| {
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
    }).collect();

    let widths = [
        Constraint::Length(14),
        Constraint::Length(20),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Surface Leases ({}) ", app.leases.leases.len())))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = TableState::default();
    state.select(Some(app.selected_row.min(app.leases.leases.len().saturating_sub(1))));
    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mode = if app.db_path.is_some() { "LOCAL" } else { "LIVE" };
    let left = format!(" {} │ {}", mode, app.connect_addr.as_str());
    let right = if let Some(ref err) = app.error_msg {
        format!(" ⚠ {} ", err)
    } else {
        let ago = app.last_refresh.elapsed().as_secs();
        format!(" Refreshed {}s ago │ r:refresh │ q:quit │ Tab:switch ", ago)
    };

    let bar = Line::from(vec![
        Span::styled(&left, Style::default().fg(Color::White).bg(Color::DarkGray)),
        Span::raw(" ".repeat(area.width as usize).chars().take(
            area.width as usize - left.len() - right.len()
        ).collect::<String>()),
        Span::styled(&right, Style::default().fg(Color::White).bg(Color::DarkGray)),
    ]);

    let status = Paragraph::new(bar).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(status, area);
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(clap::Parser)]
#[command(name = "fabric-tui", about = "TUI dashboard for Phenotype Fabric")]
struct Cli {
    /// Connect to daemon wire server at this address
    #[arg(short, long, default_value = "127.0.0.1:9400")]
    connect: String,

    /// Read from local SQLite database instead of live daemon
    #[arg(short, long)]
    db: Option<String>,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cli.connect, cli.db);
    app.refresh();

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
    terminal.draw(|frame| draw(frame, &app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                            app.running = false;
                        }
                        KeyCode::Char('q') => {
                            app.running = false;
                        }
                        KeyCode::Tab => {
                            let tabs = Tab::all();
                            let idx = tabs.iter().position(|t| *t == app.active_tab).unwrap_or(0);
                            app.active_tab = tabs[(idx + 1) % tabs.len()];
                            app.selected_row = 0;
                        }
                        KeyCode::BackTab => {
                            let tabs = Tab::all();
                            let idx = tabs.iter().position(|t| *t == app.active_tab).unwrap_or(0);
                            app.active_tab = tabs[(idx + tabs.len() - 1) % tabs.len()];
                            app.selected_row = 0;
                        }
                        KeyCode::Char('1') => { app.active_tab = Tab::Dashboard; app.selected_row = 0; }
                        KeyCode::Char('2') => { app.active_tab = Tab::Topology; app.selected_row = 0; }
                        KeyCode::Char('3') => { app.active_tab = Tab::Routes; app.selected_row = 0; }
                        KeyCode::Char('4') => { app.active_tab = Tab::Leases; app.selected_row = 0; }
                        KeyCode::Char('r') => {
                            app.refresh();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.selected_row = app.selected_row.saturating_add(1);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.selected_row = app.selected_row.saturating_sub(1);
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if !app.running {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
