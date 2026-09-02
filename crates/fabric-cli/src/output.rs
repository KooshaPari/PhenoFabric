//! Shared output formatting utilities.

use console::{style, Ansi256, Color};
use serde::Serialize;

/// Output format for CLI display.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text output (default).
    #[default]
    Text,
    /// JSON output.
    Json,
    /// YAML output.
    Yaml,
}

impl OutputFormat {
    pub fn print<T: Serialize>(&self, value: &T) -> anyhow::Result<()> {
        match self {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(value)?);
            }
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(value)?);
            }
            OutputFormat::Text => {
                // Fall through — commands handle their own text formatting.
            }
        }
        Ok(())
    }
}

/// Print a section header.
pub fn section(title: &str) {
    println!("\n{}", style(format!("── {title} ")).cyan().dim().bold());
}

/// Print a key-value pair.
pub fn kv(key: &str, value: &str) {
    print!("  {}  ", style(key).cyan());
    println!("{value}");
}

/// Print a key with a styled value.
pub fn kv_styled<F>(key: &str, f: F)
where
    F: FnOnce() -> String,
{
    print!("  {}  ", style(key).cyan());
    println!("{}", f());
}

/// Print a success message.
pub fn success(msg: &str) {
    println!("  {} {}", style("✓").green().bold(), msg);
}

/// Print an info message.
pub fn info(msg: &str) {
    println!("  {} {}", style("ℹ").blue().bold(), msg);
}

/// Print a warning message.
pub fn warning(msg: &str) {
    println!("  {} {}", style("⚠").yellow().bold(), msg);
}

/// Print a failure message.
pub fn failure(msg: &str) {
    println!("  {} {}", style("✗").red().bold(), msg);
}

/// Print a localized tier with color coding.
pub fn locality_tier(tier: &fabric_capability::LocalityTier) -> String {
    match tier.as_f64() {
        0.0 => style("L0 · same-process").green().to_string(),
        0.1..=0.3 => style("L1 · same-host").cyan().to_string(),
        0.3..=0.6 => style("L2 · same-rack").blue().to_string(),
        0.6..=0.9 => style("L3 · same-datacenter").magenta().to_string(),
        _ => style(format!("{tier}")).dim().to_string(),
    }
}

/// Print a trust level with color coding.
pub fn trust_level(level: &fabric_capability::TrustLevel) -> String {
    match level {
        fabric_capability::TrustLevel::Provided => style("provided").yellow().to_string(),
        fabric_capability::TrustLevel::Verified => style("verified").green().to_string(),
        fabric_capability::TrustLevel::Audited => style("audited").cyan().to_string(),
    }
}

/// Print a table row with aligned columns.
pub fn table_row(cols: &[&str], widths: &[usize]) {
    for (col, width) in cols.iter().zip(widths.iter()) {
        print!("{:<width$}  ", col, width = *width);
    }
    println!();
}

/// Print a horizontal rule.
pub fn rule() {
    println!("{}", style("─".repeat(60)).dim());
}
