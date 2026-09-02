// Copyright 2026 Phenotype authors
//! Workspace store and lifecycle management.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use fabric_capability::locality::LocalityTier;

use crate::error::{Error, Result};
use crate::lease::{LifecycleState, SeatLease, SeatId, TrustScope};

/// A Fabric workspace — a managed compute environment with assigned capabilities.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Unique workspace identifier.
    pub id: WorkspaceId,
    /// Human-readable name.
    pub name: String,
    /// Current lifecycle state.
    pub state: LifecycleState,
    /// All seat leases in this workspace.
    pub seats: Vec<SeatLease>,
    /// Locality tier of this workspace.
    pub locality_tier: LocalityTier,
    /// Workspace persistence file path, if any.
    pub state_file: Option<String>,
}

impl Workspace {
    /// Create a new workspace in Pending state.
    pub fn new(id: WorkspaceId, name: String, locality_tier: LocalityTier) -> Self {
        Self {
            id,
            name,
            state: LifecycleState::Pending,
            seats: Vec::new(),
            locality_tier,
            state_file: None,
        }
    }

    /// Add a seat lease to this workspace.
    pub fn add_seat(&mut self, seat: SeatLease) {
        self.seats.push(seat);
    }

    /// Remove a seat by ID.
    pub fn remove_seat(&mut self, seat_id: &SeatId) {
        self.seats.retain(|s| &s.id != seat_id);
    }

    /// Whether this workspace has any active seats.
    pub fn has_active_seats(&self) -> bool {
        self.seats.iter().any(|s| s.is_active())
    }

    /// Total number of seats.
    pub fn seat_count(&self) -> usize {
        self.seats.len()
    }

    /// Active seat count.
    pub fn active_seat_count(&self) -> usize {
        self.seats.iter().filter(|s| s.is_active()).count()
    }
}

/// Unique identifier for a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceId(pub String);

impl WorkspaceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Workspace store — manages all workspaces with JSON file persistence.
pub struct WorkspaceStore {
    /// Active workspaces keyed by ID.
    workspaces: HashMap<WorkspaceId, Workspace>,
    /// Base directory for workspace state files.
    state_dir: String,
}

impl WorkspaceStore {
    /// Open or create a workspace store at the given directory.
    pub fn open(state_dir: &Path) -> io::Result<Self> {
        if !state_dir.exists() {
            fs::create_dir_all(state_dir)?;
        }
        let mut store = Self {
            workspaces: HashMap::new(),
            state_dir: state_dir.to_string_lossy().into_owned(),
        };
        // Load existing workspaces from disk.
        for entry in fs::read_dir(state_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(workspace) = store.load_workspace(&path) {
                    store.workspaces.insert(workspace.id.clone(), workspace);
                }
            }
        }
        Ok(store)
    }

    /// List all workspace IDs.
    pub fn list(&self) -> Vec<WorkspaceId> {
        self.workspaces.keys().cloned().collect()
    }

    /// Get a workspace by ID.
    pub fn get(&self, id: &WorkspaceId) -> Option<&Workspace> {
        self.workspaces.get(id)
    }

    /// Get a mutable workspace by ID.
    pub fn get_mut(&mut self, id: &WorkspaceId) -> Option<&mut Workspace> {
        self.workspaces.get_mut(id)
    }

    /// Register a new workspace.
    pub fn create(&mut self, workspace: Workspace) -> Result<()> {
        if self.workspaces.contains_key(&workspace.id) {
            return Err(Error::Conflict {
                workspace_id: workspace.id.0.clone(),
                message: "workspace already exists".into(),
            });
        }
        self.workspaces.insert(workspace.id.clone(), workspace.clone());
        self.save_workspace(&workspace)?;
        Ok(())
    }

    /// Remove a workspace and all its seats.
    pub fn remove(&mut self, id: &WorkspaceId) -> Result<()> {
        let ws = self
            .workspaces
            .remove(id)
            .ok_or(Error::NotFound(id.0.clone()))?;

        // Release all seats.
        for seat in &ws.seats {
            if seat.is_active() {
                // Mark released; we just drop them.
            }
        }

        // Remove state file.
        if let Some(ref path) = ws.state_file {
            let file_path = Path::new(&self.state_dir).join(format!("{}.json", &ws.name));
            if file_path.exists() {
                fs::remove_file(&file_path).ok();
            }
        }
        Ok(())
    }

    /// Detect seat conflicts for a proposed lease.
    /// Returns Ok if no conflict, or Error::Conflict if a seat is already held.
    pub fn check_conflict(
        &self,
        workspace_id: &WorkspaceId,
        seat_name: &str,
    ) -> Result<()> {
        let ws = self
            .workspaces
            .get(workspace_id)
            .ok_or(Error::NotFound(workspace_id.0.clone()))?;

        for seat in &ws.seats {
            if seat.name == seat_name && seat.is_active() {
                return Err(Error::Conflict {
                    workspace_id: workspace_id.0.clone(),
                    message: format!(
                        "seat '{}' already held at {:?}",
                        seat_name, seat.locality_tier
                    ),
                });
            }
        }
        Ok(())
    }

    /// Persist a workspace to its state file.
    fn save_workspace(&self, workspace: &Workspace) -> Result<()> {
        let path = Path::new(&self.state_dir).join(format!("{}.json", &workspace.name));
        let json = serde_json::to_string_pretty(workspace)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        fs::write(&path, json).map_err(|e| Error::Io(e.to_string()))?;
        Ok(())
    }

    /// Load a workspace from a JSON file.
    fn load_workspace(&self, path: &Path) -> Result<Workspace> {
        let json = fs::read_to_string(path).map_err(|e| Error::Io(e.to_string()))?;
        serde_json::from_str(&json).map_err(|e| Error::Serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_workspace(state: LifecycleState) -> Workspace {
        Workspace {
            id: WorkspaceId::new("ws1"),
            name: "test-ws".into(),
            state,
            seats: Vec::new(),
            locality_tier: LocalityTier::L2SameAsic,
            state_file: None,
        }
    }

    #[test]
    fn test_workspace_new_is_pending() {
        let ws = make_workspace(LifecycleState::Pending);
        assert_eq!(ws.state, LifecycleState::Pending);
    }

    #[test]
    fn test_has_no_active_seats_initially() {
        let ws = make_workspace(LifecycleState::Active);
        assert!(!ws.has_active_seats());
    }

    #[test]
    fn test_seat_count_zero_initially() {
        let ws = make_workspace(LifecycleState::Active);
        assert_eq!(ws.seat_count(), 0);
    }

    #[test]
    fn test_workspace_id_equality() {
        let id1 = WorkspaceId::new("ws1");
        let id2 = WorkspaceId::new("ws1");
        let id3 = WorkspaceId::new("ws2");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_workspace_id_new() {
        let id = WorkspaceId::new("test-workspace");
        assert_eq!(id.0, "test-workspace");
    }

    #[test]
    fn test_seat_id_derivation() {
        let id = SeatLease::derive_id("ws", "gpu0");
        assert_eq!(id.0, "ws:gpu0");
    }
}
