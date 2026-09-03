//! `fabric workspace` subcommand.

use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::output;

#[derive(Args, Debug)]
pub struct CreateArgs {
    pub name: String,
    #[arg(short, long)]
    pub topology: Option<PathBuf>,
    #[arg(long, default_value = "ephemeral")]
    pub trust_scope: String,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ShowArgs {
    pub name: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    pub name: String,
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub id: String,
    pub name: String,
    pub created_at_unix: u64,
    pub trust_scope: String,
    pub topology_path: Option<String>,
}

fn index_path(workspace: &Path) -> PathBuf {
    workspace.join("workspaces").join("index.json")
}

fn workspace_path(workspace: &Path, name: &str) -> PathBuf {
    workspace.join("workspaces").join(format!("{}.json", name))
}

fn load_index(workspace: &Path) -> Result<Vec<WorkspaceState>> {
    let path = index_path(workspace);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let json = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(serde_json::from_str(&json).context("parse workspace index")?)
}

fn save_index(workspace: &Path, entries: &[WorkspaceState]) -> Result<()> {
    let path = index_path(workspace);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(entries)?;
    std::fs::write(&path, json)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn dispatch(sub: &super::WorkspaceCommand, workspace: &Path) -> Result<()> {
    match sub {
        super::WorkspaceCommand::Create(a) => create(a, workspace),
        super::WorkspaceCommand::List(a) => list(a, workspace),
        super::WorkspaceCommand::Show(a) => show(a, workspace),
        super::WorkspaceCommand::Delete(a) => delete(a, workspace),
    }
}

fn create(args: &CreateArgs, workspace: &Path) -> Result<()> {
    let id = uuid::Uuid::now_v7().to_string();
    let created_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let entry = WorkspaceState {
        id: id.clone(),
        name: args.name.clone(),
        created_at_unix,
        trust_scope: args.trust_scope.clone(),
        topology_path: args.topology.as_ref().map(|p| p.display().to_string()),
    };
    let json = serde_json::to_string_pretty(&entry)?;
    let path = workspace_path(workspace, &args.name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, json)
        .with_context(|| format!("write {}", path.display()))?;
    let mut index = load_index(workspace).unwrap_or_default();
    index.push(entry);
    save_index(workspace, &index)?;
    println!("created workspace {} ({})", args.name, id);
    Ok(())
}

fn list(args: &ListArgs, workspace: &Path) -> Result<()> {
    let index = load_index(workspace).unwrap_or_default();
    if args.json {
        let json = serde_json::to_string_pretty(&index)?;
        println!("{}", json);
    } else {
        if index.is_empty() {
            println!("(no workspaces)");
            return Ok(());
        }
        println!(
            "{:<24} {:<36} {:<16} {}",
            "NAME", "ID", "TRUST", "CREATED_UNIX"
        );
        for w in &index {
            println!(
                "{:<24} {:<36} {:<16} {}",
                w.name, w.id, w.trust_scope, w.created_at_unix
            );
        }
    }
    Ok(())
}

fn show(args: &ShowArgs, workspace: &Path) -> Result<()> {
    let path = workspace_path(workspace, &args.name);
    let json = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let entry: WorkspaceState = serde_json::from_str(&json).context("parse workspace")?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&entry)?);
    } else {
        output_pretty(&entry);
    }
    Ok(())
}

fn delete(args: &DeleteArgs, workspace: &Path) -> Result<()> {
    if !args.force {
        eprint!("delete workspace {}? [y/N] ", args.name);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("cancelled");
            return Ok(());
        }
    }
    let path = workspace_path(workspace, &args.name);
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("remove {}", path.display()))?;
    }
    let mut index = load_index(workspace).unwrap_or_default();
    index.retain(|w| w.name != args.name);
    save_index(workspace, &index)?;
    println!("deleted workspace {}", args.name);
    Ok(())
}

fn output_pretty(w: &WorkspaceState) {
    println!("{}", console::style("Workspace:").cyan().bold());
    println!("  name:         {}", w.name);
    println!("  id:           {}", w.id);
    println!("  trust_scope:  {}", w.trust_scope);
    println!("  created_unix: {}", w.created_at_unix);
    if let Some(t) = &w.topology_path {
        println!("  topology:     {}", t);
    }
}

#[allow(dead_code)]
fn _unused_output(_o: &output::OutputFormat) {}
