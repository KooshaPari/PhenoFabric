//! `fabric probe` top-level command.
//!
//! Probes the local machine capabilities and outputs the result as JSON
//! or formatted text. This is a convenience alias for `fabric cap probe`
//! with a simpler, default-oriented interface.

use anyhow::{Context, Result};
use clap::Args;

use fabric_capability::{probe::default_probe, signing};

use crate::output;

#[derive(Args, Debug)]
pub struct ProbeArgs {
    /// Output as JSON (default: pretty-printed).
    #[arg(long)]
    pub json: bool,
    /// Compact JSON (single line).
    #[arg(long)]
    pub compact: bool,
}

pub fn dispatch(args: &ProbeArgs) -> Result<()> {
    let probe = default_probe();
    let mut descriptor = probe.probe().context("capability probe failed on this host")?;

    // Auto-sign the descriptor.
    let key = signing::SigningKey::generate();
    signing::sign(&mut descriptor, &key).context("auto-sign failed")?;

    if args.compact {
        println!("{}", serde_json::to_string(&descriptor)?);
    } else if args.json {
        println!("{}", serde_json::to_string_pretty(&descriptor)?);
    } else {
        output::print_descriptor_table(&descriptor);
    }

    Ok(())
}
