//! Configuration for fabric-daemon.
//!
//! Loaded from (highest priority first):
//! 1. CLI arguments
//! 2. Config file (TOML)
//! 3. Environment variables
//! 4. Defaults

use crate::auth;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level daemon configuration.
///
/// `Default` is derived here on purpose: this impl only forwards to each field's
/// own `Default`, so it is exactly what `derive` produces and clippy's
/// `derivable_impls` (which fires from 1.98 onward) rejects a hand-written
/// version. The meaningful defaults live on the *field* structs below, which
/// keep manual impls because theirs are not derivable — `listen` is
/// `"127.0.0.1:9400"`, `path` is `"state.db"`, `wal_mode` is `true`, and so on.
///
/// Do NOT "fix" a clippy complaint about the field structs by deriving them:
/// that is what previously happened here and it silently replaced every one of
/// those values with `String::new()`/`false`/`0`, which broke first-boot
/// behaviour (an empty listen address and an empty database path).
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct DaemonConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub topology: TopologyConfig,
    pub leases: LeaseConfig,
    pub logging: LoggingConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Address to listen on (e.g. "127.0.0.1:9400").
    pub listen: String,
    /// Maximum concurrent connections.
    pub max_connections: usize,
    /// Request timeout in milliseconds.
    pub request_timeout_ms: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:9400".into(),
            max_connections: 64,
            request_timeout_ms: 5000,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Path to SQLite database file.
    pub path: PathBuf,
    /// Enable WAL mode.
    pub wal_mode: bool,
    /// Flush interval in milliseconds (0 = synchronous).
    pub flush_interval_ms: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("state.db"),
            wal_mode: true,
            flush_interval_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TopologyConfig {
    /// Auto-probe topology on startup.
    pub auto_probe: bool,
    /// Probe interval in seconds.
    pub probe_interval_s: u64,
    /// Persist epoch across restarts.
    pub epoch_persistence: bool,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        Self {
            auto_probe: true,
            probe_interval_s: 30,
            epoch_persistence: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct LeaseConfig {
    /// Default lease TTL in seconds.
    pub default_ttl_s: u64,
    /// Maximum lease TTL in seconds.
    pub max_ttl_s: u64,
    /// Renewal window before expiry in seconds.
    pub renewal_window_s: u64,
    /// Fairness policy name.
    pub fairness_policy: String,
}

impl Default for LeaseConfig {
    fn default() -> Self {
        Self {
            default_ttl_s: 3600,
            max_ttl_s: 86400,
            renewal_window_s: 300,
            fairness_policy: "FairShare".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error).
    pub level: String,
    /// Log format (json, pretty, compact).
    pub format: String,
    /// Optional log file path.
    pub file: Option<PathBuf>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
            format: "pretty".into(),
            file: None,
        }
    }
}

/// Authentication configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Whether authentication is enabled.
    pub enabled: bool,
    /// WorkOS OAuth client ID.
    pub workos_client_id: String,
    /// WorkOS OAuth client secret. Loaded from env var `WORKOS_CLIENT_SECRET`.
    pub workos_client_secret: String,
    /// WorkOS OAuth redirect URI.
    pub workos_redirect_uri: String,
    /// Infisical service account client ID.
    pub infisical_client_id: String,
    /// Infisical service account client secret. Loaded from env var `INFISICAL_CLIENT_SECRET`.
    pub infisical_client_secret: String,
    /// Infisical project ID.
    pub infisical_project_id: String,
    /// JWT secret for local token decoding (alternative to WorkOS JWKS verification).
    pub jwt_secret: Option<String>,
    /// Message types that are exempt from authentication.
    pub public_routes: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            workos_client_id: String::new(),
            workos_client_secret: String::new(),
            workos_redirect_uri: String::new(),
            infisical_client_id: String::new(),
            infisical_client_secret: String::new(),
            infisical_project_id: String::new(),
            jwt_secret: None,
            // Auth bootstrap messages must be reachable before a user is
            // authenticated; everything else requires a token.
            public_routes: vec![
                "health_check".into(),
                "status_check".into(),
                "auth_start".into(),
                "auth_complete".into(),
                "auth_email".into(),
                "auth_status".into(),
                "auth_verify".into(),
            ],
        }
    }
}

impl From<AuthConfig> for auth::AuthMiddlewareConfig {
    fn from(config: AuthConfig) -> Self {
        let public_routes: std::collections::HashSet<String> =
            config.public_routes.into_iter().collect();

        let workos_config = if !config.workos_client_id.is_empty() {
            Some(auth::WorkOsConfig {
                client_id: config.workos_client_id,
                client_secret: config.workos_client_secret,
                redirect_uri: config.workos_redirect_uri,
                ..Default::default()
            })
        } else {
            None
        };

        Self {
            enabled: config.enabled,
            public_routes,
            workos_config,
            jwt_secret: config.jwt_secret,
        }
    }
}

/// Configuration for federation (multi-node topology sharing).
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FederationConfig {
    /// Whether federation is enabled.
    pub enabled: bool,
    /// Sync interval in seconds.
    pub sync_interval_s: u64,
    /// List of peer daemon addresses (e.g. ["10.0.0.2:9400"]).
    pub peers: Vec<String>,
    /// Merge strategy name.
    pub merge_strategy: String,
}

#[allow(dead_code)]
impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sync_interval_s: 30,
            peers: Vec::new(),
            merge_strategy: "MergeAll".into(),
        }
    }
}

impl DaemonConfig {
    /// Load environment variables into sensitive config fields.
    pub fn load_env_secrets(&mut self) {
        if let Ok(secret) = std::env::var("WORKOS_CLIENT_SECRET") {
            self.auth.workos_client_secret = secret;
        }
        // The client id and redirect URI are not secrets, but they are
        // deployment-specific: a desktop build passes them to the daemon it
        // spawns so login works without a config file on disk.
        if let Ok(client_id) = std::env::var("WORKOS_CLIENT_ID") {
            self.auth.workos_client_id = client_id;
        }
        if let Ok(redirect_uri) = std::env::var("WORKOS_REDIRECT_URI") {
            self.auth.workos_redirect_uri = redirect_uri;
        }
        if let Ok(secret) = std::env::var("INFISICAL_CLIENT_SECRET") {
            self.auth.infisical_client_secret = secret;
        }
    }

    /// Save the configuration to a TOML file.
    pub fn save(&self, path: &std::path::Path) -> Result<(), ConfigError> {
        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Serialize(e.to_string()))?;

        // Ensure parent directory exists.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::Io(e.to_string()))?;
        }

        std::fs::write(path, content).map_err(|e| ConfigError::Io(e.to_string()))
    }

    /// Load configuration from a TOML file.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigError> {
        let content =
            std::fs::read_to_string(path.as_ref()).map_err(|e| ConfigError::Io(e.to_string()))?;
        let config: DaemonConfig =
            toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;
        Ok(config)
    }

    /// Merge with CLI overrides (None = keep file value).
    pub fn with_overrides(
        mut self,
        listen: Option<String>,
        db_path: Option<PathBuf>,
        log_level: Option<String>,
    ) -> Self {
        if let Some(l) = listen {
            self.server.listen = l;
        }
        if let Some(p) = db_path {
            self.database.path = p;
        }
        if let Some(level) = log_level {
            self.logging.level = level;
        }
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("serialize error: {0}")]
    Serialize(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = DaemonConfig::default();
        assert_eq!(config.server.listen, "127.0.0.1:9400");
        assert_eq!(config.database.path, PathBuf::from("state.db"));
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn parse_toml_config() {
        let toml_str = r#"
[server]
listen = "0.0.0.0:8080"
max_connections = 128

[database]
path = "/tmp/test.db"

[logging]
level = "debug"
format = "json"
"#;
        let config: DaemonConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.listen, "0.0.0.0:8080");
        assert_eq!(config.server.max_connections, 128);
        assert_eq!(config.database.path, PathBuf::from("/tmp/test.db"));
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.logging.format, "json");
    }

    #[test]
    fn with_overrides() {
        let config = DaemonConfig::default().with_overrides(
            Some("0.0.0.0:3000".into()),
            Some(PathBuf::from("/custom.db")),
            Some("trace".into()),
        );
        assert_eq!(config.server.listen, "0.0.0.0:3000");
        assert_eq!(config.database.path, PathBuf::from("/custom.db"));
        assert_eq!(config.logging.level, "trace");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("daemon.toml");

        let mut config = DaemonConfig::default();
        config.server.listen = "0.0.0.0:5555".into();
        config.logging.level = "debug".into();

        config.save(&path).unwrap();
        let loaded = DaemonConfig::from_file(&path).unwrap();

        assert_eq!(loaded.server.listen, "0.0.0.0:5555");
        assert_eq!(loaded.logging.level, "debug");
        assert_eq!(loaded.database.path, PathBuf::from("state.db"));
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("subdir").join("daemon.toml");

        let config = DaemonConfig::default();
        config.save(&path).unwrap();
        assert!(path.exists());
    }
}
