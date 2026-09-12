//! fabric-gui: egui-based native GUI for Phenotype Fabric.
//!
//! Provides topology visualization, route management, and daemon control
//! in a native desktop window. Can also serve as a Parsec streaming target.

use std::path::{Path, PathBuf};

use fabric_graph::model::{Topology, TrustLevel};
use fabric_tray::DaemonStatus;

/// Application state for the GUI.
pub struct FabricGui {
    /// Current sidebar selection.
    pub panel: Panel,
    /// Workspace path.
    pub workspace: PathBuf,
    /// Loaded topology.
    pub topology: Option<Topology>,
    /// Node count (cached).
    pub node_count: usize,
    /// Edge count (cached).
    pub edge_count: usize,
    /// Capability count.
    pub cap_count: usize,
    /// Route plans.
    pub routes: Vec<RouteEntry>,
    /// Daemon health status.
    pub daemon_status: DaemonStatus,
    /// Status message.
    pub status_msg: String,
    /// Selected node index.
    pub selected_node: Option<usize>,
    /// Selected edge index.
    pub selected_edge: Option<usize>,
    /// Selected route index.
    pub selected_route: Option<usize>,
}

/// Sidebar panel selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Topology,
    Routes,
    Capabilities,
    Health,
}

/// Route plan entry.
#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub intent: String,
    pub steps: usize,
    pub cost: f64,
    pub trust_level: String,
    pub file_name: String,
}

impl FabricGui {
    /// Create a new GUI application instance.
    pub fn new(workspace: &Path) -> Self {
        Self {
            panel: Panel::Topology,
            workspace: workspace.to_path_buf(),
            topology: None,
            node_count: 0,
            edge_count: 0,
            cap_count: 0,
            routes: Vec::new(),
            daemon_status: DaemonStatus::Stopped,
            status_msg: String::from("Ready"),
            selected_node: None,
            selected_edge: None,
            selected_route: None,
        }
    }

    /// Load workspace data from disk.
    pub fn load_data(&mut self) {
        // Load topology
        let topo_path = self.workspace.join("topology.json");
        if topo_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&topo_path) {
                if let Ok(topo) = serde_json::from_str::<Topology>(&content) {
                    self.node_count = topo.nodes.len();
                    self.edge_count = topo.edges.len();
                    self.topology = Some(topo);
                    self.status_msg = format!(
                        "Loaded topology: {} nodes, {} edges",
                        self.node_count, self.edge_count
                    );
                } else {
                    self.status_msg = "Failed to parse topology.json".to_string();
                }
            } else {
                self.status_msg = "Failed to read topology.json".to_string();
            }
        } else {
            self.status_msg = "No topology.json found in workspace".to_string();
        }

        // Load route plans
        self.routes.clear();
        let routes_dir = self.workspace.join("routes");
        if routes_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&routes_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "json") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(plan) =
                                serde_json::from_str::<serde_json::Value>(&content)
                            {
                                self.routes.push(RouteEntry {
                                    intent: plan
                                        .get("intent")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                    steps: plan
                                        .get("steps")
                                        .and_then(|v| v.as_array())
                                        .map_or(0, |a| a.len()),
                                    cost: plan
                                        .get("total_cost")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0),
                                    trust_level: plan
                                        .get("trust_level")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                    file_name: path
                                        .file_stem()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Count capabilities
        let caps_dir = self.workspace.join("capabilities");
        if caps_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&caps_dir) {
                self.cap_count = entries
                    .flatten()
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                    .count();
            }
        }
    }
}

/// Format a trust level as a display string.
pub fn trust_label(level: TrustLevel) -> &'static str {
    match level {
        TrustLevel::Untrusted => "Untrusted",
        TrustLevel::Bootstrap => "Bootstrap",
        TrustLevel::Attested => "Attested",
        TrustLevel::Audited => "Audited",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_label_all_variants() {
        assert_eq!(trust_label(TrustLevel::Untrusted), "Untrusted");
        assert_eq!(trust_label(TrustLevel::Bootstrap), "Bootstrap");
        assert_eq!(trust_label(TrustLevel::Attested), "Attested");
        assert_eq!(trust_label(TrustLevel::Audited), "Audited");
    }

    #[test]
    fn fabric_gui_new_sets_defaults() {
        let app = FabricGui::new(Path::new("/tmp/test-workspace"));
        assert_eq!(app.panel, Panel::Topology);
        assert_eq!(app.node_count, 0);
        assert!(app.topology.is_none());
        assert!(app.routes.is_empty());
        assert_eq!(app.daemon_status, DaemonStatus::Stopped);
    }

    #[test]
    fn load_data_missing_workspace_is_ok() {
        let mut app = FabricGui::new(Path::new("/nonexistent/path"));
        app.load_data();
        assert!(app.topology.is_none());
        assert!(app.routes.is_empty());
        assert_eq!(app.cap_count, 0);
    }
}
