//! Output formatting helpers for the fabric CLI.
use anyhow::Result;
use fabric_capability::CapabilityDescriptor;
use fabric_capability::locality::LocalityTier;
use fabric_graph::model::TrustLevel;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    JsonPretty,
}

impl OutputFormat {
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json | Self::JsonPretty)
    }
}

pub fn resolve_format(json: bool, pretty: bool) -> OutputFormat {
    if pretty {
        OutputFormat::JsonPretty
    } else if json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    }
}

pub fn emit<T, F>(format: OutputFormat, value: &T, text_fn: F) -> Result<()>
where
    T: Serialize,
    F: FnOnce() -> String,
{
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string(value)?),
        OutputFormat::JsonPretty => println!("{}", serde_json::to_string_pretty(value)?),
        OutputFormat::Text => print!("{}", text_fn()),
    }
    Ok(())
}

pub fn print_descriptor_table(desc: &CapabilityDescriptor) {
    println!(
        "{:<18} {}",
        console::style("Node ID:").cyan().bold(),
        desc.node_id
    );
    println!(
        "{:<18} {}",
        console::style("Epoch:").cyan().bold(),
        desc.epoch
    );
    println!(
        "{:<18} {}",
        console::style("Schema:").cyan().bold(),
        desc.schema_version
    );
    println!(
        "{:<18} {}",
        console::style("Probed:").cyan().bold(),
        desc.probed_at.to_rfc3339()
    );
    println!(
        "{:<18} {}",
        console::style("Probe ver:").cyan().bold(),
        desc.probe_version
    );
    println!(
        "{:<18} {}",
        console::style("Topology hash:").cyan().bold(),
        desc.topology_hash
    );
    println!(
        "{:<18} {}",
        console::style("Signatures:").cyan().bold(),
        desc.signatures.len()
    );
    println!();
    println!("{}", console::style("Capabilities").green().bold());
    println!("  {:#?}", desc.capabilities);
}

pub fn print_topology_summary(name: &str, node_count: usize, edge_count: usize) {
    println!(
        "{} {} ({} nodes, {} edges)",
        console::style("Topology:").cyan().bold(),
        name,
        node_count,
        edge_count
    );
}

pub fn format_tier(tier: LocalityTier) -> String {
    let code = tier.short_code();
    let numeric = tier.as_f64();
    format!("{} ({:.1})", code, numeric)
}

pub fn format_trust(level: TrustLevel) -> String {
    let name = match level {
        TrustLevel::Untrusted => "untrusted",
        TrustLevel::Bootstrap => "bootstrap",
        TrustLevel::Attested => "attested",
        TrustLevel::Audited => "audited",
    };
    match level {
        TrustLevel::Untrusted => console::style(name).red().to_string(),
        TrustLevel::Bootstrap => console::style(name).yellow().to_string(),
        TrustLevel::Attested => console::style(name).green().to_string(),
        TrustLevel::Audited => console::style(name).cyan().to_string(),
    }
}
