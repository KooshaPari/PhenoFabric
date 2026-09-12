//! Phenotype Fabric reference surface CLI.
//!
//! PF-WP-020.06 deliverable — primary user-facing interface for Fabric.

use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use fabric_cli::{Cli, Commands, commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = match cli.verbose {
        0 => EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("warn")),
        1 => EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info")),
        2 => EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("debug")),
        _ => EnvFilter::new("trace"),
    };

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_level(true))
        .with(filter)
        .init();

    // Resolve workspace path
    let workspace = cli.workspace.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".fabric")
    });

    // Dispatch
    let result = match &cli.command {
        Commands::Cap { sub } => commands::cap::dispatch(sub, &workspace),
        Commands::Graph { sub } => commands::graph::dispatch(sub, &workspace),
        Commands::Route { sub } => commands::route::dispatch(sub, &workspace),
        Commands::Workspace { sub } => commands::workspace::dispatch(sub, &workspace),
        Commands::Tui => {
            tracing_subscriber::registry()
                .with(fmt::layer().with_target(true).with_level(true))
                .with(EnvFilter::new("info"))
                .init();
            fabric_cli::tui::run(&workspace)
        }
    };

    if let Err(ref e) = result {
        if !cli.quiet {
            eprintln!("{}: {}", console::style("error").red().bold(), e);
            for cause in std::iter::successors(e.source(), |e| e.source()) {
                eprintln!("  {}: cause: {}", console::style("↳").dim(), cause);
            }
        }
        std::process::exit(1);
    }

    Ok(())
}
