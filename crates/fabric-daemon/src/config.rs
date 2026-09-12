//! Configuration for fabric-daemon.
//!
//! Loaded from (highest priority first):
//! 1. CLI arguments
//! 2. Config file (TOML)
//! 3. Environment variables
//! 4. Defaults

use serde::Deserialize;
use std::path::PathBuf;

/// Top-level daemon configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub topology: TopologyConfig,
    pub leases: LeaseConfig,
    pub logging: LoggingConfig,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            topology: TopologyConfig::default(),
            leases: LeaseConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

impl DaemonConfig {
    /// Load configuration from a TOML file.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::Io(e.to_string()))?;
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
}
