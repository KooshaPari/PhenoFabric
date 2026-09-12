//! Core coordinator logic for fabric-daemon.
//!
//! The coordinator owns the in-memory state (topology, leases, plans)
//! and orchestrates persistence via fabric-persist.

use fabric_graph::model::{RoutePlan, Topology, TopologyEpoch};
use fabric_graph::surface::SurfaceLease;
use fabric_persist::Persist;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::info;

use crate::config::DaemonConfig;
use crate::health::HealthResponse;

/// The main coordinator state, shared across threads.
pub struct Coordinator {
    /// Shared daemon state behind a mutex for thread-safe access.
    state: Mutex<CoordinatorState>,
    /// Persistence layer.
    persist: Persist,
    /// Shutdown flag.
    shutdown: Arc<AtomicBool>,
    /// When the daemon started.
    start_time: Instant,
    /// Configuration.
    config: DaemonConfig,
}

/// Mutable coordinator state.
struct CoordinatorState {
    topology: Topology,
    active_leases: Vec<SurfaceLease>,
    active_plans: Vec<RoutePlan>,
    dirty: bool,
}

impl Coordinator {
    /// Create a new coordinator, recovering state from the database.
    pub fn new(config: DaemonConfig) -> Result<Self, CoordinatorError> {
        let persist = Persist::open(&config.database.path)
            .map_err(|e| CoordinatorError::Database(e.to_string()))?;

        let recovered = persist
            .recover_state()
            .map_err(|e| CoordinatorError::Recovery(e.to_string()))?;

        info!(
            topology_nodes = recovered.topology.nodes.len(),
            topology_edges = recovered.topology.edges.len(),
            active_leases = recovered.active_leases.len(),
            active_plans = recovered.active_plans.len(),
            "state recovered from database"
        );

        let state = CoordinatorState {
            topology: recovered.topology,
            active_leases: recovered.active_leases,
            active_plans: recovered.active_plans,
            dirty: false,
        };

        Ok(Self {
            state: Mutex::new(state),
            persist,
            shutdown: Arc::new(AtomicBool::new(false)),
            start_time: Instant::now(),
            config,
        })
    }

    /// Build a health response from current state.
    pub fn health(&self) -> HealthResponse {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        HealthResponse::new(
            self.start_time,
            state.topology.epoch.0,
            state.active_leases.len(),
            state.active_plans.len(),
        )
    }

    /// Replace the topology (e.g. after a probe cycle).
    pub fn set_topology(&self, topo: Topology) -> Result<(), CoordinatorError> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.topology = topo;
        state.dirty = true;
        Ok(())
    }

    /// Get a snapshot of the current topology epoch.
    pub fn topology_epoch(&self) -> TopologyEpoch {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.topology.epoch
    }

    /// Flush dirty state to SQLite (topology + leases + plans).
    pub fn flush(&self) -> Result<(), CoordinatorError> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.dirty {
            return Ok(());
        }
        self.persist
            .save_topology(&state.topology)
            .map_err(|e| CoordinatorError::Flush(e.to_string()))?;
        for lease in &state.active_leases {
            self.persist
                .save_lease(lease)
                .map_err(|e| CoordinatorError::Flush(e.to_string()))?;
        }
        for plan in &state.active_plans {
            self.persist
                .save_route_plan(plan)
                .map_err(|e| CoordinatorError::Flush(e.to_string()))?;
        }
        state.dirty = false;
        info!(
            topology = %state.topology.meta.name,
            leases = state.active_leases.len(),
            plans = state.active_plans.len(),
            "state flushed to database"
        );
        Ok(())
    }

    /// Check if shutdown has been requested.
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    /// Request graceful shutdown.
    pub fn shutdown(&self) {
        info!("shutdown requested");
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Get a reference to the shutdown flag (for sharing with signal handler).
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        self.shutdown.clone()
    }

    /// Compile a multihop route using the current in-memory topology.
    ///
    /// Locks the state mutex, clones the topology, releases the lock,
    /// then runs `compile_multihop`. This avoids holding the lock during
    /// the (potentially expensive) compilation.
    pub fn compile_multihop(
        &self,
        source: &fabric_graph::model::NodeId,
        destination: &fabric_graph::model::NodeId,
        intent: &fabric_graph::model::Intent,
        catalog: &[fabric_graph::multihop::TransportStage],
    ) -> Result<fabric_graph::multihop::MultihopResult, CoordinatorError> {
        let topo = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.topology.clone()
        };
        fabric_graph::multihop::compile_multihop(&topo, source, destination, intent, catalog)
            .map_err(|e| CoordinatorError::Compile(e.to_string()))
    }

    /// Insert a lease into the active set.
    pub fn insert_lease(&self, lease: SurfaceLease) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.active_leases.push(lease);
        state.dirty = true;
    }

    /// Insert a route plan into the active set.
    pub fn insert_plan(&self, plan: RoutePlan) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.active_plans.push(plan);
        state.dirty = true;
    }

    /// Mark a node as failed. Removes it from the topology and
    /// returns the count of affected leases.
    pub fn mark_node_failed(&self, node_id: &fabric_graph::model::NodeId) -> usize {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        // Remove the node from the topology.
        state.topology.nodes.remove(node_id);
        // Remove edges touching the failed node.
        state
            .topology
            .edges
            .retain(|_id, e| e.from != *node_id && e.to != *node_id);
        // Count affected leases (leases whose current binding touches this node).
        let affected = state
            .active_leases
            .iter()
            .filter(|l| {
                l.current
                    .as_ref()
                    .map_or(false, |b| b.step_node == *node_id)
            })
            .count();
        state.dirty = true;
        affected
    }

    /// Get the listen address.
    pub fn listen_addr(&self) -> &str {
        &self.config.server.listen
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CoordinatorError {
    #[error("database error: {0}")]
    Database(String),
    #[error("recovery error: {0}")]
    Recovery(String),
    #[error("flush error: {0}")]
    Flush(String),
    #[error("compile error: {0}")]
    Compile(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> (DaemonConfig, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        (config, dir)
    }

    #[test]
    fn coordinator_new_recovers_empty_state() {
        let (config, _dir) = test_config();
        let coord = Coordinator::new(config).unwrap();
        let health = coord.health();
        assert_eq!(health.status, "healthy");
        assert_eq!(health.active_leases, 0);
        assert_eq!(health.active_plans, 0);
    }

    #[test]
    fn coordinator_shutdown() {
        let (config, _dir) = test_config();
        let coord = Coordinator::new(config).unwrap();
        assert!(!coord.is_shutting_down());
        coord.shutdown();
        assert!(coord.is_shutting_down());
    }

    #[test]
    fn coordinator_flush_noop_when_clean() {
        let (config, _dir) = test_config();
        let coord = Coordinator::new(config).unwrap();
        coord.flush().unwrap();
    }

    #[test]
    fn coordinator_topology_epoch() {
        let (config, _dir) = test_config();
        let coord = Coordinator::new(config).unwrap();
        let epoch = coord.topology_epoch();
        assert_eq!(epoch.0, 0);
    }

    #[test]
    fn coordinator_insert_lease_marks_dirty() {
        let (config, _dir) = test_config();
        let coord = Coordinator::new(config).unwrap();
        assert!(!coord.state.lock().unwrap().dirty);
        let lease = fabric_graph::surface::SurfaceLease {
            handle: fabric_graph::surface::SurfaceHandle::new(),
            spec: fabric_graph::surface::SurfaceSpec {
                name: "test-surface".into(),
                protocol: fabric_graph::surface::SurfaceProtocol::Custom("test".into()),
                capture: None,
                locality_floor: fabric_graph::LocalityTier::L5Loopback,
                refresh_hz: None,
                audio_sample_rate_hz: None,
                requires_rt_island: false,
                strict_epoch_binding: false,
                min_host_trust: fabric_graph::TrustLevel::Untrusted,
                expires_at: None,
            },
            current: None,
            history: vec![],
            state: fabric_graph::surface::LeaseState::Active,
            exit_reason: None,
            created_at: chrono::Utc::now(),
            terminated_at: None,
        };
        coord.insert_lease(lease);
        assert!(coord.state.lock().unwrap().dirty);
        assert_eq!(coord.state.lock().unwrap().active_leases.len(), 1);
    }

    #[test]
    fn coordinator_insert_plan_marks_dirty() {
        let (config, _dir) = test_config();
        let coord = Coordinator::new(config).unwrap();
        let topo = fabric_graph::Topology::new();
        let intent = fabric_graph::builder::IntentBuilder::new()
            .name("test")
            .min_trust(fabric_graph::TrustLevel::Untrusted)
            .build();
        if let Ok(plan) = fabric_graph::compile(&topo, &intent) {
            coord.insert_plan(plan);
            assert!(coord.state.lock().unwrap().dirty);
            assert_eq!(coord.state.lock().unwrap().active_plans.len(), 1);
        }
    }

    #[test]
    fn coordinator_mark_node_failed() {
        let (config, _dir) = test_config();
        let coord = Coordinator::new(config).unwrap();
        let topo = fabric_graph::builder::TopologyBuilder::new()
            .with_name("test")
            .add(fabric_graph::Node::new(
                fabric_graph::model::NodeId::new("n1"),
                fabric_graph::LocalityTier::L5Loopback,
            ))
            .add(fabric_graph::Node::new(
                fabric_graph::model::NodeId::new("n2"),
                fabric_graph::LocalityTier::L5Loopback,
            ))
            .connect("n1", "n2", fabric_graph::LocalityTier::L1SameNuma)
            .build();
        coord.set_topology(topo).unwrap();
        let affected = coord.mark_node_failed(&fabric_graph::model::NodeId::new("n1"));
        assert_eq!(affected, 0); // no leases touch n1
        assert!(!coord.state.lock().unwrap().topology.nodes.contains_key(&fabric_graph::model::NodeId::new("n1")));
    }
}
