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
    ///
    /// A replacement whose runtime-only secret fields are empty (a
    /// `config_snapshot` round-trip drops them via `skip_serializing`, so
    /// they deserialize as empty) keeps the values this session already
    /// loaded; an explicitly supplied nonempty secret still wins.
    pub fn update_config(&self, mut new_config: DaemonConfig) {
        {
            let mut cfg = self.config.lock().unwrap_or_else(|e| e.into_inner());
            if new_config.auth.workos_client_secret.is_empty() {
                new_config.auth.workos_client_secret = cfg.auth.workos_client_secret.clone();
            }
            if new_config.auth.infisical_client_secret.is_empty() {
                new_config.auth.infisical_client_secret = cfg.auth.infisical_client_secret.clone();
            }
            *cfg = new_config;
        }
        self.persist_config();
    }

    /// Apply partial config overrides (feature toggles) on top of current config.
    /// If a field is present in `overrides`, it replaces the current value.
    pub fn apply_config_overrides(&self, overrides: &serde_json::Value) -> Result<(), String> {
        let mut cfg = self.config.lock().unwrap_or_else(|e| e.into_inner());
        apply_config_overrides_inner(&mut cfg, overrides)?;
        let path = self
            .config_path
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
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
        let path = self
            .config_path
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, DatabaseConfig};

    fn coord_with_secrets(dir: &std::path::Path) -> Coordinator {
        Coordinator::new(DaemonConfig {
            database: DatabaseConfig {
                path: dir.join("test.db"),
                ..Default::default()
            },
            auth: AuthConfig {
                enabled: true,
                workos_client_id: "client_abc".into(),
                workos_client_secret: "WORKOS-SECRET-NEVER-LEAKED".into(),
                infisical_client_secret: "INFISICAL-SECRET-NEVER-LEAKED".into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap()
    }

    #[test]
    fn config_snapshot_omits_secret_fields() {
        let dir = tempfile::tempdir().unwrap();
        let coord = coord_with_secrets(dir.path());
        let snapshot = coord.config_snapshot();

        assert!(
            !snapshot.contains("workos_client_secret"),
            "field leaked: {snapshot}"
        );
        assert!(
            !snapshot.contains("infisical_client_secret"),
            "field leaked: {snapshot}"
        );
        assert!(
            !snapshot.contains("WORKOS-SECRET-NEVER-LEAKED"),
            "value leaked: {snapshot}"
        );
        assert!(
            !snapshot.contains("INFISICAL-SECRET-NEVER-LEAKED"),
            "value leaked: {snapshot}"
        );
        // Non-secret auth config still present in the snapshot.
        assert!(
            snapshot.contains("client_abc"),
            "client id missing: {snapshot}"
        );
    }

    #[test]
    fn override_persist_omits_secret_fields() {
        // cmd_start flow: set_config_path + apply_config_overrides -> cfg.save.
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("daemon.toml");
        let coord = coord_with_secrets(dir.path());
        coord.set_config_path(cfg_path.clone());

        coord
            .apply_config_overrides(&serde_json::json!({"server": {"listen": "0.0.0.0:9999"}}))
            .unwrap();

        let content = std::fs::read_to_string(&cfg_path).unwrap();
        assert!(
            !content.contains("workos_client_secret"),
            "field persisted: {content}"
        );
        assert!(
            !content.contains("infisical_client_secret"),
            "field persisted: {content}"
        );
        assert!(
            !content.contains("WORKOS-SECRET-NEVER-LEAKED"),
            "value persisted: {content}"
        );
        assert!(
            !content.contains("INFISICAL-SECRET-NEVER-LEAKED"),
            "value persisted: {content}"
        );
        assert!(
            content.contains("0.0.0.0:9999"),
            "override not persisted: {content}"
        );
    }

    #[test]
    fn update_config_preserves_runtime_secrets_when_snapshot_omits_them() {
        // handle_save_config path (wire/handlers.rs): the client round-trips a
        // config_snapshot, whose skip_serializing fields deserialize as empty.
        // That full replacement must not wipe the runtime secrets this session
        // is running with (CodeRabbit actionable, 2026-09-27).
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("daemon.toml");
        let coord = coord_with_secrets(dir.path());
        coord.set_config_path(cfg_path.clone());

        let snapshot = coord.config_snapshot();
        let replacement: DaemonConfig =
            serde_json::from_str(&snapshot).expect("snapshot must deserialize as a full config");
        assert!(
            replacement.auth.workos_client_secret.is_empty(),
            "snapshot must omit the secret fields"
        );

        coord.update_config(replacement);

        let cfg = coord.config.lock().unwrap();
        assert_eq!(
            cfg.auth.workos_client_secret, "WORKOS-SECRET-NEVER-LEAKED",
            "save_config replacement wiped the runtime WorkOS secret"
        );
        assert_eq!(
            cfg.auth.infisical_client_secret, "INFISICAL-SECRET-NEVER-LEAKED",
            "save_config replacement wiped the runtime Infisical secret"
        );
        drop(cfg);

        // Persistence stays runtime-only: the rewritten file still omits both.
        let content = std::fs::read_to_string(&cfg_path).unwrap();
        assert!(
            !content.contains("WORKOS-SECRET-NEVER-LEAKED"),
            "value persisted: {content}"
        );
        assert!(
            !content.contains("INFISICAL-SECRET-NEVER-LEAKED"),
            "value persisted: {content}"
        );
    }
}
