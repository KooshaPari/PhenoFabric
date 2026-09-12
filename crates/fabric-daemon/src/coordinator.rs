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

    /// Flush dirty state to SQLite.
    pub fn flush(&self) -> Result<(), CoordinatorError> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.dirty {
            return Ok(());
        }
        self.persist
            .save_topology(&state.topology)
            .map_err(|e| CoordinatorError::Flush(e.to_string()))?;
        state.dirty = false;
        info!("state flushed to database");
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
}
