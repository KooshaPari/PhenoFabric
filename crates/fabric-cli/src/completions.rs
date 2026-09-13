//! Shell completion generation for the fabric CLI.
//!
//! Uses `clap_complete` to generate completion scripts for bash, zsh, and fish.

use clap::ValueEnum;
use std::fmt;

/// Supported shell types for completions.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Shell::Bash => write!(f, "bash"),
            Shell::Zsh => write!(f, "zsh"),
            Shell::Fish => write!(f, "fish"),
        }
    }
}

/// Generate a shell completion script for the given shell type.
///
/// Returns the completion script as a string suitable for writing to stdout.
pub fn generate_completion(shell: Shell) -> String {
    // We build a Cli instance without subcommand parsing for completion generation.
    // The actual Cli type is in fabric_cli, but we use clap_complete directly with
    // a Command so we don't pull in the full dependency tree here.
    let mut cmd = build_cli_command();
    let mut buf = std::io::BufWriter::new(Vec::new());

    match shell {
        Shell::Bash => {
            clap_complete::generate(
                clap_complete::Shell::Bash,
                &mut cmd,
                "fabric",
                &mut buf,
            );
        }
        Shell::Zsh => {
            clap_complete::generate(
                clap_complete::Shell::Zsh,
                &mut cmd,
                "fabric",
                &mut buf,
            );
        }
        Shell::Fish => {
            clap_complete::generate(
                clap_complete::Shell::Fish,
                &mut cmd,
                "fabric",
                &mut buf,
            );
        }
    }

    let inner = buf.into_inner().unwrap_or_default();
    String::from_utf8(inner).unwrap_or_default()
}

/// Build the clap Command definition matching `fabric-cli`'s CLI structure.
///
/// This mirrors `fabric_cli::Cli` without depending on the full crate.
fn build_cli_command() -> clap::Command {
    use clap::{arg, value_parser, Command};

    Command::new("fabric")
        .about("Phenotype Fabric reference surface CLI")
        .arg(
            arg!(-q --quiet "Suppress all output except errors"),
        )
        .arg(
            clap::Arg::new("verbose")
                .short('v')
                .action(clap::ArgAction::Count)
                .help("Enable verbose output (vv for trace-level)"),
        )
        .arg(
            arg!(-w --workspace <PATH> "Path to the workspace directory (default: ~/.fabric/)"),
        )
        .subcommand(Command::new("cap").about("Probe and inspect machine capabilities"))
        .subcommand(Command::new("graph").about("Build and inspect the capability topology graph"))
        .subcommand(Command::new("route").about("Compile routes and manage route plans"))
        .subcommand(Command::new("workspace").about("Manage named workspaces"))
        .subcommand(Command::new("surface").about("Manage surface leases (displays, audio, network endpoints)"))
        .subcommand(Command::new("probe").about("Probe local machine capabilities (top-level, outputs JSON)"))
        .subcommand(Command::new("status").about("Show daemon health status"))
        .subcommand(Command::new("check").about("Run checker against a manifest"))
        .subcommand(Command::new("tui").about("Launch the interactive TUI topology explorer"))
        .subcommand(Command::new("completions").about("Generate shell completion scripts").arg(
            clap::arg!(-s --shell <SHELL> "Shell type (bash, zsh, fish)")
                .required(true)
                .value_parser(value_parser!(Shell)),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_completion_is_valid() {
        let script = generate_completion(Shell::Bash);
        assert!(!script.is_empty(), "bash completion should not be empty");
        // Bash completions always contain this function pattern.
        assert!(
            script.contains("fabric") || script.contains("complete"),
            "bash completion should reference 'fabric'"
        );
    }

    #[test]
    fn zsh_completion_is_valid() {
        let script = generate_completion(Shell::Zsh);
        assert!(!script.is_empty(), "zsh completion should not be empty");
        assert!(
            script.contains("fabric") || script.contains("compdef") || script.contains("compadd"),
            "zsh completion should reference fabric or use compdef/compadd"
        );
    }

    #[test]
    fn fish_completion_is_valid() {
        let script = generate_completion(Shell::Fish);
        assert!(!script.is_empty(), "fish completion should not be empty");
        assert!(
            script.contains("fabric") || script.contains("complete"),
            "fish completion should reference 'fabric'"
        );
    }

    #[test]
    fn shell_display_roundtrip() {
        assert_eq!(Shell::Bash.to_string(), "bash");
        assert_eq!(Shell::Zsh.to_string(), "zsh");
        assert_eq!(Shell::Fish.to_string(), "fish");
    }
}
