//! Interactive TUI topology explorer for Phenotype Fabric.
//!
//! Launched via `fabric tui`. Provides tabbed views of the workspace:
//! topology graph, route plans, capabilities, and daemon health.

pub mod views;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Tabs};
use ratatui::Terminal;

use fabric_graph::model::Topology;

/// Tabs in the TUI explorer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Topology,
    Routes,
    Capabilities,
    Health,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Topology, Tab::Routes, Tab::Capabilities, Tab::Health]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Topology => "Topology",
            Tab::Routes => "Routes",
            Tab::Capabilities => "Capabilities",
            Tab::Health => "Health",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Topology => 0,
            Tab::Routes => 1,
            Tab::Capabilities => 2,
            Tab::Health => 3,
        }
    }
}

/// Route plan entry for display.
#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub intent: String,
    pub steps: usize,
    pub cost: f64,
    pub trust_level: String,
}

/// Workspace data loaded from disk.
#[derive(Debug, Default)]
pub struct WorkspaceData {
    pub topology: Option<Topology>,
    pub routes: Vec<RouteEntry>,
    pub cap_count: usize,
    pub node_count: usize,
    pub edge_count: usize,
}

/// TUI application state.
pub struct App {
    /// Current tab.
    pub tab: Tab,
    /// Selected row index.
    pub selected: usize,
    /// Workspace path.
    pub workspace: PathBuf,
    /// Loaded workspace data.
    pub data: WorkspaceData,
    /// Whether to quit.
    pub quit: bool,
    /// Scroll offset for large lists.
    pub scroll: usize,
    /// Daemon health status (polled periodically).
    pub daemon_healthy: bool,
}

impl App {
    fn new(workspace: &Path) -> Self {
        Self {
            tab: Tab::Topology,
            selected: 0,
            workspace: workspace.to_path_buf(),
            data: WorkspaceData::default(),
            quit: false,
            scroll: 0,
            daemon_healthy: false,
        }
    }

    /// Load workspace data from disk.
    fn load_data(&mut self) {
        // Load topology if it exists
        let topo_path = self.workspace.join("topology.json");
        if topo_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&topo_path) {
                if let Ok(topo) = serde_json::from_str::<Topology>(&content) {
                    self.data.node_count = topo.nodes.len();
                    self.data.edge_count = topo.edges.len();
                    self.data.topology = Some(topo);
                }
            }
        }

        // Load route plans
        let routes_dir = self.workspace.join("routes");
        if routes_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&routes_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "json") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(plan) = serde_json::from_str::<serde_json::Value>(&content) {
                                self.data.routes.push(RouteEntry {
                                    intent: plan
                                        .get("intent")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                    steps: plan
                                        .get("steps")
                                        .and_then(|v| v.as_array())
                                        .map_or(0, |a| a.len()),
                                    cost: plan
                                        .get("total_cost")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0),
                                    trust_level: plan
                                        .get("trust_level")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Count capability descriptors
        let caps_dir = self.workspace.join("capabilities");
        if caps_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&caps_dir) {
                self.data.cap_count = entries
                    .flatten()
                    .filter(|e| e.path().extension().map_or(false, |e| e == "json"))
                    .count();
            }
        }
    }

    /// Move to next tab.
    fn next_tab(&mut self) {
        let tabs = Tab::all();
        let idx = self.tab.index();
        self.tab = tabs[(idx + 1) % tabs.len()];
        self.selected = 0;
        self.scroll = 0;
    }

    /// Move to previous tab.
    fn prev_tab(&mut self) {
        let tabs = Tab::all();
        let idx = self.tab.index();
        self.tab = tabs[(idx + tabs.len() - 1) % tabs.len()];
        self.selected = 0;
        self.scroll = 0;
    }

    /// Current list length for selection bounds.
    fn list_len(&self) -> usize {
        match self.tab {
            Tab::Topology => self.data.node_count,
            Tab::Routes => self.data.routes.len(),
            Tab::Capabilities => self.data.cap_count,
            Tab::Health => 4,
        }
    }

    /// Move selection up.
    fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down.
    fn select_down(&mut self) {
        let len = self.list_len();
        if len > 0 && self.selected < len - 1 {
            self.selected += 1;
        }
    }
}

/// Run the TUI event loop.
pub fn run(workspace: &Path) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(workspace);
    app.load_data();

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            let area = f.area();

            // Main layout: tabs + content + status bar
            let chunks = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Tabs
                    Constraint::Min(1),   // Content
                    Constraint::Length(1), // Status bar
                ])
                .split(area);

            // Tab bar
            let tab_titles: Vec<Line> = Tab::all()
                .iter()
                .map(|t| {
                    let style = if *t == app.tab {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    Line::from(Span::styled(t.title(), style))
                })
                .collect();

            let tabs_widget = Tabs::new(tab_titles)
                .block(Block::default().borders(Borders::BOTTOM).title("Fabric Explorer"))
                .select(app.tab.index())
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_widget(tabs_widget, chunks[0]);

            // Content area
            match app.tab {
                Tab::Topology => views::render_topology(f, chunks[1], &app),
                Tab::Routes => views::render_routes(f, chunks[1], &app),
                Tab::Capabilities => views::render_capabilities(f, chunks[1], &app),
                Tab::Health => views::render_health(f, chunks[1], &app),
            }

            // Status bar
            let daemon_status = if app.daemon_healthy {
                Span::styled(" ● Daemon: healthy", Style::default().fg(Color::Green))
            } else {
                Span::styled(" ● Daemon: offline", Style::default().fg(Color::Red))
            };

            let status_line = Line::from(vec![
                Span::styled(
                    format!(" Workspace: {}", app.workspace.display()),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" │"),
                Span::styled(
                    format!(" Nodes: {} │ Edges: {} ", app.data.node_count, app.data.edge_count),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" │"),
                daemon_status,
                Span::raw(" │ q:quit ←→:tabs ↑↓:select r:refresh"),
            ]);
            f.render_widget(status_line, chunks[2]);
        })?;

        // Handle input
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                handle_key(&mut app, key);
            }
        }

        // Periodic tick (for health polling, etc.)
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            // TODO: poll daemon health via TCP
        }

        if app.quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

/// Handle a key event.
fn handle_key(app: &mut App, key: KeyEvent) {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit = true;
        return;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
        KeyCode::Tab => app.next_tab(),
        KeyCode::BackTab => app.prev_tab(),
        KeyCode::Up | KeyCode::Char('k') => app.select_up(),
        KeyCode::Down | KeyCode::Char('j') => app.select_down(),
        KeyCode::Home => app.selected = 0,
        KeyCode::End => {
            let len = app.list_len();
            if len > 0 {
                app.selected = len - 1;
            }
        }
        KeyCode::Char('r') => app.load_data(),
        _ => {}
    }
}
