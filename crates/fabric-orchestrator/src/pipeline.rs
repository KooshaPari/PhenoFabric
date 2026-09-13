//! Core `FabricPipeline` that wires all Fabric components into a single
//! running process.
//!
//! The pipeline owns:
//! - Coordinator (topology, leases, plans, wire dispatch)
//! - Persist (SQLite durability)
//! - Surface registry (active surface leases)
//! - Health tracker

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use fabric_daemon::config::DaemonConfig;
use fabric_daemon::coordinator::Coordinator;
use fabric_daemon::health::HealthResponse;
use fabric_graph::surface_runtime::SurfaceRegistry;
use tracing::info;

/// Snapshot of the full pipeline status.
#[derive(Debug, serde::Serialize)]
pub struct PipelineStatus {
    /// Daemon health info.
    pub health: HealthResponse,
    /// Number of registered surfaces.
    pub surface_count: usize,
    /// Uptime in seconds.
    pub uptime_s: u64,
}

/// The orchestrator pipeline. Holds every subsystem and exposes
/// lifecycle and query methods.
pub struct FabricPipeline {
    /// The coordinator (wraps persist internally).
    pub(crate) coordinator: Arc<Coordinator>,
    /// In-memory surface registry.
    surface_registry: SurfaceRegistry,
    /// Shutdown flag (shared with coordinator).
    shutdown_flag: Arc<AtomicBool>,
    /// Pipeline start time.
    start_time: Instant,
    /// Configuration (kept for read-back).
    config: DaemonConfig,
}

impl FabricPipeline {
    /// Create a new pipeline from configuration.
    ///
    /// Opens the database, recovers state, and wires up the coordinator.
    pub fn new(config: DaemonConfig) -> Result<Self, PipelineError> {
        let coordinator = Coordinator::new(config.clone())
            .map_err(|e| PipelineError::Coordinator(e.to_string()))?;

        let shutdown_flag = coordinator.shutdown_flag();
        let coordinator = Arc::new(coordinator);

        info!("fabric-pipeline created");

        Ok(Self {
            coordinator,
            surface_registry: SurfaceRegistry::new(),
            shutdown_flag,
            start_time: Instant::now(),
            config,
        })
    }

    /// Start the pipeline (mark running). This is a no-op for now; actual
    /// wire-server binding happens in `serve::start_wire_server`.
    pub fn start(&self) -> Result<(), PipelineError> {
        info!(
            listen = %self.config.server.listen,
            db = %self.config.database.path.display(),
            "pipeline starting"
        );
        Ok(())
    }

    /// Request graceful shutdown of the entire pipeline.
    pub fn stop(&self) {
        info!("pipeline stop requested");
        self.coordinator.shutdown();
    }

    /// Return true if the pipeline has been asked to shut down.
    pub fn is_shutting_down(&self) -> bool {
        self.coordinator.is_shutting_down()
    }

    /// Compile a topology from a JSON file path.
    ///
    /// Loads the topology JSON, sets it on the coordinator, then runs
    /// a compile pass. Returns the resulting route plans as JSON.
    pub fn compile_topology(
        &self,
        topology_path: &std::path::Path,
    ) -> Result<serde_json::Value, PipelineError> {
        let content = std::fs::read_to_string(topology_path)
            .map_err(|e| PipelineError::Io(e.to_string()))?;
        let topo: fabric_graph::Topology = serde_json::from_str(&content)
            .map_err(|e| PipelineError::Parse(e.to_string()))?;

        self.coordinator
            .set_topology(topo)
            .map_err(|e| PipelineError::Coordinator(e.to_string()))?;

        let plans_json = self.coordinator.plans_snapshot();
        let plans: serde_json::Value = serde_json::from_str(&plans_json)
            .map_err(|e| PipelineError::Parse(e.to_string()))?;

        Ok(plans)
    }

    /// Build a full status dump.
    pub fn get_status(&self) -> PipelineStatus {
        let health = self.coordinator.health();
        PipelineStatus {
            surface_count: self.surface_registry.len(),
            uptime_s: self.start_time.elapsed().as_secs(),
            health,
        }
    }

    /// Get a reference to the surface registry.
    pub fn surface_registry(&self) -> &SurfaceRegistry {
        &self.surface_registry
    }

    /// Get a mutable reference to the surface registry.
    pub fn surface_registry_mut(&mut self) -> &mut SurfaceRegistry {
        &mut self.surface_registry
    }

    /// Get a reference to the coordinator.
    pub fn coordinator(&self) -> &Arc<Coordinator> {
        &self.coordinator
    }

    /// Flush dirty state to the database.
    pub fn flush(&self) -> Result<(), PipelineError> {
        self.coordinator
            .flush()
            .map_err(|e| PipelineError::Coordinator(e.to_string()))
    }
}

/// Errors originating from the pipeline.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("coordinator error: {0}")]
    Coordinator(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> (DaemonConfig, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("pipeline_test.db");
        let config = DaemonConfig {
            database: fabric_daemon::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        (config, dir)
    }

    #[test]
    fn pipeline_new_succeeds() {
        let (config, _dir) = test_config();
        let pipeline = FabricPipeline::new(config).unwrap();
        assert!(!pipeline.is_shutting_down());
    }

    #[test]
    fn pipeline_stop_sets_shutdown() {
        let (config, _dir) = test_config();
        let pipeline = FabricPipeline::new(config).unwrap();
        pipeline.stop();
        assert!(pipeline.is_shutting_down());
    }

    #[test]
    fn pipeline_get_status_returns_healthy() {
        let (config, _dir) = test_config();
        let pipeline = FabricPipeline::new(config).unwrap();
        let status = pipeline.get_status();
        assert_eq!(status.health.status, "healthy");
        assert_eq!(status.surface_count, 0);
    }

    #[test]
    fn pipeline_compile_empty_topology() {
        let (config, _dir) = test_config();
        let pipeline = FabricPipeline::new(config).unwrap();

        // Write an empty topology to disk.
        let topo_path = _dir.path().join("empty_topo.json");
        std::fs::write(&topo_path, r#"{"epoch":0,"nodes":{},"edges":{},"meta":{"name":"empty","annotations":{}}}"#).unwrap();

        let result = pipeline.compile_topology(&topo_path);
        assert!(result.is_ok(), "compile failed: {result:?}");
        // Plans should be empty for empty topology.
        let plans = result.unwrap();
        assert_eq!(plans["routes"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn pipeline_compile_nonexistent_file_fails() {
        let (config, _dir) = test_config();
        let pipeline = FabricPipeline::new(config).unwrap();
        let result = pipeline.compile_topology(std::path::Path::new("/nonexistent/topo.json"));
        assert!(result.is_err());
    }
}
