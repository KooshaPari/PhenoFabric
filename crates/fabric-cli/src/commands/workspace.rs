//! `fabric workspace` subcommand.
//!
//! Manages named workspaces — persistent compute environments with seat leases.

use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Workspace name.
    #[arg(long)]
    pub name: String,
    /// Locality tier for the workspace (e.g. "L0SameProcess", "L5Loopback").
    #[arg(long, default_value = "L5Loopback")]
    pub tier: String,
    /// Optional topology file to associate with the workspace.
    #[arg(short, long)]
    pub topology: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ShowArgs {
    /// Workspace name or ID.
    pub name: String,
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Workspace name or ID.
    pub name: String,
    /// Skip confirmation prompt.
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct ReleaseArgs {
    /// Workspace name or ID to release (releases all seat leases).
    pub id: String,
    /// Skip confirmation prompt.
    #[arg(short, long)]
    pub force: bool,
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub id: String,
    pub name: String,
    pub created_at_unix: u64,
    pub tier: String,
    pub topology_path: Option<String>,
    /// Seat leases held by this workspace.
    #[serde(default)]
    pub seats: Vec<SeatState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatState {
    pub seat_id: String,
    pub name: String,
    pub state: String,
    pub locality_tier: String,
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
    serde_json::from_str::<Vec<WorkspaceState>>(&json)
        .with_context(|| format!("parse workspace index from {}", path.display()))
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

pub fn dispatch(sub: &crate::WorkspaceCommand, workspace: &Path) -> Result<()> {
    match sub {
        crate::WorkspaceCommand::Create(a) => create(a, workspace),
        crate::WorkspaceCommand::List(a) => list(a, workspace),
        crate::WorkspaceCommand::Show(a) => show(a, workspace),
        crate::WorkspaceCommand::Delete(a) => delete(a, workspace),
        crate::WorkspaceCommand::Release(a) => release(a, workspace),
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
        tier: args.tier.clone(),
        topology_path: args.topology.as_ref().map(|p| p.display().to_string()),
        seats: Vec::new(),
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
        println!("{{:<24}} {{:<36}} {{:<18}} {{:<10}} CREATED_UNIX");
        for w in &index {
            println!(
                "{:<24} {:<36} {:<18} {:<10} {}",
                w.name,
                w.id,
                w.tier,
                w.seats.len(),
                w.created_at_unix
            );
        }
    }
    Ok(())
}

fn show(args: &ShowArgs, workspace: &Path) -> Result<()> {
    // Try to find by name first, then by ID.
    let path = workspace_path(workspace, &args.name);
    let entry = if path.exists() {
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))?;
        serde_json::from_str::<WorkspaceState>(&json).context("parse workspace")?
    } else {
        // Search by ID in the index.
        let index = load_index(workspace).unwrap_or_default();
        index
            .into_iter()
            .find(|w| w.id == args.name)
            .ok_or_else(|| anyhow::anyhow!("workspace '{}' not found", args.name))?
    };

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

    // Try to find workspace by name or ID.
    let ws_name = find_workspace_name(workspace, &args.name)?;
    let path = workspace_path(workspace, &ws_name);
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("remove {}", path.display()))?;
    }
    let mut index = load_index(workspace).unwrap_or_default();
    index.retain(|w| w.name != ws_name && w.id != args.name);
    save_index(workspace, &index)?;
    println!("deleted workspace {}", ws_name);
    Ok(())
}

fn release(args: &ReleaseArgs, workspace: &Path) -> Result<()> {
    if !args.force {
        eprint!("release workspace {} and all its seats? [y/N] ", args.id);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("cancelled");
            return Ok(());
        }
    }

    let ws_name = find_workspace_name(workspace, &args.id)?;
    let path = workspace_path(workspace, &ws_name);

    let mut entry: WorkspaceState = if path.exists() {
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))?;
        serde_json::from_str(&json).context("parse workspace")?
    } else {
        anyhow::bail!("workspace '{}' not found", args.id);
    };

    let seat_count = entry.seats.len();
    let released_count = entry
        .seats
        .iter_mut()
        .filter(|s| s.state == "active" || s.state == "pending")
        .map(|s| {
            s.state = "released".to_string();
            1
        })
        .sum::<usize>();

    // Save updated workspace.
    let json = serde_json::to_string_pretty(&entry)?;
    std::fs::write(&path, json)
        .with_context(|| format!("write {}", path.display()))?;

    // Update index.
    let mut index = load_index(workspace).unwrap_or_default();
    for w in &mut index {
        if w.name == ws_name || w.id == args.id {
            w.seats = entry.seats.clone();
        }
    }
    save_index(workspace, &index)?;

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "workspace": ws_name,
                "id": entry.id,
                "total_seats": seat_count,
                "released_seats": released_count,
            })
        );
    } else {
        println!(
            "{} released workspace {} ({} seats released, {} total)",
            console::style("ok").green().bold(),
            ws_name,
            released_count,
            seat_count,
        );
    }

    Ok(())
}

/// Find the workspace name by name or ID.
fn find_workspace_name(workspace: &Path, name_or_id: &str) -> Result<String> {
    let path = workspace_path(workspace, name_or_id);
    if path.exists() {
        return Ok(name_or_id.to_string());
    }
    let index = load_index(workspace).unwrap_or_default();
    index
        .iter()
        .find(|w| w.id == name_or_id)
        .map(|w| w.name.clone())
        .ok_or_else(|| anyhow::anyhow!("workspace '{}' not found", name_or_id))
}

fn output_pretty(w: &WorkspaceState) {
    println!("{}", console::style("Workspace:").cyan().bold());
    println!("  name:   {}", w.name);
    println!("  id:     {}", w.id);
    println!("  tier:   {}", w.tier);
    println!("  created: {}", w.created_at_unix);
    if let Some(t) = &w.topology_path {
        println!("  topology: {}", t);
    }
    if w.seats.is_empty() {
        println!("  seats:  (none)");
    } else {
        println!("  seats:");
        for seat in &w.seats {
            let state_color = match seat.state.as_str() {
                "active" => console::style(&seat.state).green(),
                "released" | "revoked" => console::style(&seat.state).red(),
                "pending" => console::style(&seat.state).yellow(),
                _ => console::style(&seat.state),
            };
            println!(
                "    {:<20} {:<20} {}",
                seat.name, seat.seat_id, state_color
            );
        }
    }
}
