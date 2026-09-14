//! Core config update/persist/override methods for the Coordinator.

use std::path::PathBuf;

use tracing::{info, warn};

use crate::config::DaemonConfig;

use super::Coordinator;

/// Apply partial config overrides (feature toggles) on top of current config.
/// If a field is present in `overrides`, it replaces the current value.
pub(super) fn apply_config_overrides_inner(
    config: &mut DaemonConfig,
    overrides: &serde_json::Value,
) -> Result<(), String> {
    if let Some(server) = overrides.get("server") {
        if let Some(v) = server.get("listen").and_then(|v| v.as_str()) {
            config.server.listen = v.to_string();
        }
        if let Some(v) = server.get("max_connections").and_then(|v| v.as_u64()) {
            config.server.max_connections = v as usize;
        }
        if let Some(v) = server.get("request_timeout_ms").and_then(|v| v.as_u64()) {
            config.server.request_timeout_ms = v;
        }
    }
    if let Some(topo) = overrides.get("topology") {
        if let Some(v) = topo.get("auto_probe").and_then(|v| v.as_bool()) {
            config.topology.auto_probe = v;
        }
        if let Some(v) = topo.get("probe_interval_s").and_then(|v| v.as_u64()) {
            config.topology.probe_interval_s = v;
        }
        if let Some(v) = topo.get("epoch_persistence").and_then(|v| v.as_bool()) {
            config.topology.epoch_persistence = v;
        }
    }
    if let Some(leases) = overrides.get("leases") {
        if let Some(v) = leases.get("default_ttl_s").and_then(|v| v.as_u64()) {
            config.leases.default_ttl_s = v;
        }
        if let Some(v) = leases.get("max_ttl_s").and_then(|v| v.as_u64()) {
            config.leases.max_ttl_s = v;
        }
        if let Some(v) = leases.get("renewal_window_s").and_then(|v| v.as_u64()) {
            config.leases.renewal_window_s = v;
        }
        if let Some(v) = leases.get("fairness_policy").and_then(|v| v.as_str()) {
            config.leases.fairness_policy = v.to_string();
        }
    }
    if let Some(logging) = overrides.get("logging") {
        if let Some(v) = logging.get("level").and_then(|v| v.as_str()) {
            config.logging.level = v.to_string();
        }
        if let Some(v) = logging.get("format").and_then(|v| v.as_str()) {
            config.logging.format = v.to_string();
        }
    }
    Ok(())
}

impl Coordinator {
    /// Set the path for config file persistence.
    pub fn set_config_path(&self, path: PathBuf) {
        *self.config_path.lock().unwrap_or_else(|e| e.into_inner()) = Some(path);
    }

    /// Update configuration and optionally persist to disk.
    pub fn update_config(&self, new_config: DaemonConfig) {
        {
            let mut cfg = self.config.lock().unwrap_or_else(|e| e.into_inner());
            *cfg = new_config;
        }
        self.persist_config();
    }

    /// Apply partial config overrides (feature toggles) on top of current config.
    /// If a field is present in `overrides`, it replaces the current value.
    pub fn apply_config_overrides(&self, overrides: &serde_json::Value) -> Result<(), String> {
        let mut cfg = self.config.lock().unwrap_or_else(|e| e.into_inner());
        apply_config_overrides_inner(&mut cfg, overrides)?;
        let path = self.config_path.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let Some(path) = path {
            if let Err(e) = cfg.save(&path) {
                warn!(error = %e, "failed to persist config after override");
            } else {
                info!(path = %path.display(), "config persisted to disk");
            }
        }
        Ok(())
    }

    /// Return the current configuration as JSON.
    pub fn config_snapshot(&self) -> String {
        let cfg = self.config.lock().unwrap_or_else(|e| e.into_inner());
        serde_json::to_string(&*cfg).unwrap_or_else(|_| r"{}".into())
    }

    /// Persist the current config to disk if a config path is set.
    pub(super) fn persist_config(&self) {
        let path = self.config_path.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let Some(path) = path {
            let cfg = self.config.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = cfg.save(&path) {
                warn!(error = %e, "failed to persist config");
            } else {
                info!(path = %path.display(), "config persisted to disk");
            }
        }
    }
}
