//! `fabric check` command.
//!
//! Runs the fabric-checker against a manifest to determine whether the
//! local machine satisfies the application's requirements.

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

use fabric_capability::probe::default_probe;
use fabric_checker::{self, CheckerManifest};

use crate::output;

#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Path to the checker manifest file (JSON or YAML).
    #[arg(short, long)]
    pub manifest: PathBuf,
    /// Also probe local capabilities and output the descriptor.
    #[arg(long)]
    pub probe: bool,
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
    /// Pretty-print JSON output.
    #[arg(long)]
    pub pretty: bool,
}

pub fn dispatch(args: &CheckArgs) -> Result<()> {
    // Load manifest.
    let text = std::fs::read_to_string(&args.manifest)
        .with_context(|| format!("read {}", args.manifest.display()))?;
    let manifest: CheckerManifest = if args.manifest.extension().is_some_and(|e| e == "yaml" || e == "yml") {
        serde_yaml::from_str(&text).context("parse manifest YAML")?
    } else {
        serde_json::from_str(&text).context("parse manifest JSON")?
    };

    // Probe local capabilities.
    let probe = default_probe();
    let descriptor = probe.probe().context("capability probe failed on this host")?;

    // Run the checker.
    let decision = fabric_checker::check(&descriptor, &manifest);

    let format = output::resolve_format(args.json, args.pretty);
    output::emit(format, &decision, || {
        let mut out = String::new();
        match &decision {
            fabric_checker::Decision::Admit => {
                out.push_str(&format!(
                    "{} machine satisfies manifest requirements\n",
                    console::style("ADMIT").green().bold(),
                ));
            }
            fabric_checker::Decision::AdmitWithNotes { notes } => {
                out.push_str(&format!(
                    "{} machine satisfies manifest requirements (with notes)\n\n",
                    console::style("ADMIT WITH NOTES").yellow().bold(),
                ));
                for note in notes {
                    out.push_str(&format!(
                        "  {}: {}\n",
                        console::style(format!("[{:?}]", note.code)).yellow(),
                        note.message,
                    ));
                }
            }
            fabric_checker::Decision::Reject {
                reason_code,
                reason_message,
            } => {
                out.push_str(&format!(
                    "{} machine does NOT satisfy manifest requirements\n\n",
                    console::style("REJECT").red().bold(),
                ));
                out.push_str(&format!(
                    "  {}: {}\n",
                    console::style(format!("[{:?}]", reason_code)).red(),
                    reason_message,
                ));
            }
        }
        out.push_str(&format!(
            "\n{} node_id: {}\n",
            console::style("Probe:").cyan().bold(),
            descriptor.node_id,
        ));
        out.push_str(&format!(
            "{} {}\n",
            console::style("Probed at:").cyan().bold(),
            descriptor.probed_at.to_rfc3339(),
        ));
        if args.probe {
            out.push_str(&format!(
                "\n{}\n{}",
                console::style("Descriptor:").cyan().bold(),
                serde_json::to_string_pretty(&descriptor).unwrap_or_default(),
            ));
        }
        out
    })?;

    // Exit with non-zero status on Reject.
    if matches!(decision, fabric_checker::Decision::Reject { .. }) {
        std::process::exit(1);
    }

    Ok(())
}
