//! fabric-tui: Terminal dashboard for Phenotype Fabric daemon.
//!
//! Displays topology, routes, surface leases, and health in a ratatui TUI.
//! Connects to the daemon wire server via TCP for live data, or reads from
//! a local SQLite database.

mod app;
mod types;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use clap::Parser;
use ratatui::prelude::*;

use app::{App, Tab};
use ui::draw;

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
                        KeyCode::Char('q')
                        | KeyCode::Char('c')
                            if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                        {
                            app.running = false;
                        }
                        KeyCode::Char('q') => {
                            app.running = false;
                        }
                        KeyCode::Tab => {
                            let tabs = Tab::all();
                            let idx = tabs
                                .iter()
                                .position(|t| *t == app.active_tab)
                                .unwrap_or(0);
                            app.active_tab = tabs[(idx + 1) % tabs.len()];
                            app.selected_row = 0;
                        }
                        KeyCode::BackTab => {
                            let tabs = Tab::all();
                            let idx = tabs
                                .iter()
                                .position(|t| *t == app.active_tab)
                                .unwrap_or(0);
                            app.active_tab = tabs[(idx + tabs.len() - 1) % tabs.len()];
                            app.selected_row = 0;
                        }
                        KeyCode::Char('1') => {
                            app.active_tab = Tab::Dashboard;
                            app.selected_row = 0;
                        }
                        KeyCode::Char('2') => {
                            app.active_tab = Tab::Topology;
                            app.selected_row = 0;
                        }
                        KeyCode::Char('3') => {
                            app.active_tab = Tab::Routes;
                            app.selected_row = 0;
                        }
                        KeyCode::Char('4') => {
                            app.active_tab = Tab::Leases;
                            app.selected_row = 0;
                        }
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
