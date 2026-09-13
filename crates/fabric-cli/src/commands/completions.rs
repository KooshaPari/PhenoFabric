//! `fabric completions` subcommand.
//!
//! Outputs shell completion scripts to stdout for bash, zsh, or fish.

use clap::Args;

use crate::completions::{generate_completion, Shell};

#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Shell to generate completions for (bash, zsh, fish).
    #[arg(short, long)]
    pub shell: Shell,
}

/// Execute the completions subcommand.
pub fn execute(args: &CompletionsArgs) -> anyhow::Result<()> {
    let script = generate_completion(args.shell);
    print!("{script}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completions_bash_prints_script() {
        let args = CompletionsArgs { shell: Shell::Bash };
        // execute() prints to stdout; just verify it doesn't panic.
        execute(&args).unwrap();
    }

    #[test]
    fn completions_zsh_prints_script() {
        let args = CompletionsArgs { shell: Shell::Zsh };
        execute(&args).unwrap();
    }

    #[test]
    fn completions_fish_prints_script() {
        let args = CompletionsArgs { shell: Shell::Fish };
        execute(&args).unwrap();
    }
}
