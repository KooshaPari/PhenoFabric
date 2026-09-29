//! Infisical secrets manager client for fabric-daemon.
//!
//! Fetches, stores, and manages secrets via the Infisical API:
//! - Client credentials authentication
//! - Secret retrieval by path and environment
//! - Secret storage and updates

#![allow(dead_code)]

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Default Infisical API base URL (US Cloud), without the `/api` prefix
/// that the client appends to every route.
///
/// Evidence (observed 2026-09-24): the installed Infisical CLI v0.43.114
/// ships `--domain` default `https://app.infisical.com/api` ("Required for
/// non-US Cloud users"), and the general API reference uses the same host.
/// The v4 endpoint source separately states `https://us.infisical.com`, so
/// the two official statements conflict — recorded as an ambiguity in
/// `docs/runbooks/workos-infisical-shared.md`. The former default
/// `https://secrets.infisical.com` is neither documented nor a documented
/// self-hosted placeholder. Override per deployment with
/// `INFISICAL_BASE_URL`.
const DEFAULT_INFISICAL_BASE_URL: &str = "https://app.infisical.com";

/// Errors that can occur during secrets operations.
#[derive(Debug, Error)]
pub enum SecretsError {
    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("authentication failed: {0}")]
    Auth(String),

    #[error("secret not found: {0}")]
    NotFound(String),

    #[error("secret operation failed: {0}")]
    OperationFailed(String),

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl SecretsError {
    pub(super) fn http(err: reqwest::Error) -> Self {
        Self::Http(err.to_string())
    }

    fn _serialization(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

/// Configuration for the Infisical secrets client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfisicalConfig {
    /// Service account client ID.
    pub client_id: String,
    /// Service account client secret.
    pub client_secret: String,
    /// Infisical project ID.
    pub project_id: String,
    /// Infisical API base URL. Defaults to `https://app.infisical.com`.
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_base_url() -> String {
    DEFAULT_INFISICAL_BASE_URL.to_string()
}

impl Default for InfisicalConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            project_id: String::new(),
            base_url: DEFAULT_INFISICAL_BASE_URL.to_string(),
        }
    }
}

/// A secret value with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretValue {
    /// The secret key (name).
    pub key: String,
    /// The secret value.
    pub value: String,
    /// The environment this secret belongs to.
    pub environment: String,
    /// Optional secret path.
    #[serde(default)]
    pub path: Option<String>,
}

/// Response from the Infisical token endpoint.
///
/// Field names follow the official login contract (docs observed
/// 2026-09-27, https://infisical.com/docs/api-reference/endpoints/universal-auth/login):
/// `accessToken` / `expiresIn` / `tokenType`.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresIn")]
    expires_in: u64,
    // Infisical returns `tokenType`; map it explicitly. Never read (the
    // leading underscore marks it ignorable), but it MUST deserialize or the
    // login response fails to parse and every fetch reports `Unavailable`.
    #[serde(rename = "tokenType")]
    _token_type: String,
}

/// Response from the Infisical secrets list endpoint.
#[derive(Debug, Deserialize)]
struct SecretsListResponse {
    secrets: Vec<InfisicalSecret>,
}

/// A single secret from the Infisical API.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct InfisicalSecret {
    id: String,
    key: String,
    value: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    version: Option<u64>,
    #[serde(default)]
    tags: Vec<SecretTag>,
}

/// A secret tag.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct SecretTag {
    id: String,
    name: String,
}

/// Infisical secrets manager client.
///
/// Authenticates via client credentials (service account) and provides
/// methods for reading and writing secrets.
#[derive(Debug, Clone)]
pub struct InfisicalClient {
    pub(super) config: InfisicalConfig,
    pub(super) http: Client,
    /// Cached access token, if available.
    token: Option<CachedToken>,
}

/// A cached access token with expiry.
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    /// Instant when the token expires.
    expires_at: std::time::Instant,
}

impl InfisicalClient {
    /// Create a new Infisical client.
    pub fn new(config: InfisicalConfig) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to create HTTP client");
        Self {
            config,
            http,
            token: None,
        }
    }

    /// Create a new Infisical client with a custom HTTP client (for testing).
    pub fn with_client(config: InfisicalConfig, http: Client) -> Self {
        Self {
            config,
            http,
            token: None,
        }
    }

    /// Authenticate and obtain an access token using client credentials.
    ///
    /// Request/response field names follow the official contract (docs
    /// observed 2026-09-27). Live Infisical enforces the request shape:
    /// sending `client_id`/`client_secret` returns 422
    /// `path:["clientId"] Required` (observed 2026-09-27).
    ///
    /// The token is cached internally for subsequent requests.
    pub async fn authenticate(&mut self) -> Result<String, SecretsError> {
        let url = format!("{}/api/v1/auth/universal-auth/login", self.config.base_url);

        let mut body = HashMap::new();
        body.insert("clientId", &self.config.client_id);
        body.insert("clientSecret", &self.config.client_secret);

        let response = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(SecretsError::http)?;

        if !response.status().is_success() {
            let status = response.status();
            let resp_body = response.text().await.unwrap_or_default();
            return Err(SecretsError::Auth(format!("HTTP {status}: {resp_body}")));
        }

        let token_data: TokenResponse = response.json().await.map_err(SecretsError::http)?;

        let expires_at = std::time::Instant::now()
            + std::time::Duration::from_secs(token_data.expires_in.saturating_sub(60));

        self.token = Some(CachedToken {
            access_token: token_data.access_token.clone(),
            expires_at,
        });

        Ok(token_data.access_token)
    }

    /// Get a valid access token, refreshing if necessary.
    pub(super) async fn ensure_token(&mut self) -> Result<String, SecretsError> {
        if let Some(ref cached) = self.token {
            if std::time::Instant::now() < cached.expires_at {
                return Ok(cached.access_token.clone());
            }
        }
        self.authenticate().await
    }

    /// Fetch a single secret by key path and environment (legacy v1 raw
    /// read, retained for compatibility). Folder-scoped named-secret reads
    /// use [`Self::get_secret_by_name`] (official v4) instead; the v1
    /// folder semantics are UNKNOWN pending evidence.
    ///
    /// # Arguments
    /// * `path` - The secret path (e.g., "/database/password")
    /// * `env` - The environment slug (e.g., "prod", "dev")
    pub async fn get_secret(&mut self, path: &str, env: &str) -> Result<SecretValue, SecretsError> {
        let token = self.ensure_token().await?;

        let url = format!(
            "{}/api/v1/secrets/raw/{}",
            self.config.base_url,
            path.trim_start_matches('/')
        );

        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .query(&[
                ("environment", env),
                ("project_id", &self.config.project_id),
            ])
            .send()
            .await
            .map_err(SecretsError::http)?;

        if response.status().as_u16() == 404 {
            return Err(SecretsError::NotFound(format!(
                "secret at path '{path}' in environment '{env}'"
            )));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SecretsError::OperationFailed(format!(
                "HTTP {status}: {body}"
            )));
        }

        let secret_data: serde_json::Value = response.json().await.map_err(SecretsError::http)?;

        parse_secret_payload(&secret_data, path, env, Some(path.to_string()))
    }

    /// Fetch all secrets for a given environment.
    ///
    /// # Arguments
    /// * `env` - The environment slug (e.g., "prod", "dev")
    pub async fn get_secrets(&mut self, env: &str) -> Result<Vec<SecretValue>, SecretsError> {
        let token = self.ensure_token().await?;

        let url = format!("{}/api/v1/secrets/raw", self.config.base_url);

        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .query(&[
                ("environment", env),
                ("project_id", &self.config.project_id),
            ])
            .send()
            .await
            .map_err(SecretsError::http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SecretsError::OperationFailed(format!(
                "HTTP {status}: {body}"
            )));
        }

        let data: SecretsListResponse = response.json().await.map_err(SecretsError::http)?;

        Ok(data
            .secrets
            .into_iter()
            .map(|s| SecretValue {
                key: s.key,
                value: s.value,
                environment: env.to_string(),
                path: s.path,
            })
            .collect())
    }

    /// Store or update a secret.
    ///
    /// # Arguments
    /// * `key` - The secret key (name)
    /// * `value` - The secret value
    /// * `env` - The environment slug
    pub async fn set_secret(
        &mut self,
        key: &str,
        value: &str,
        env: &str,
    ) -> Result<(), SecretsError> {
        let token = self.ensure_token().await?;

        let url = format!("{}/api/v1/secrets/raw", self.config.base_url);

        let mut body = HashMap::new();
        body.insert("secretKey", key);
        body.insert("secretValue", value);
        body.insert("environment", env);
        body.insert("project_id", &self.config.project_id);

        let response = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(SecretsError::http)?;

        if !response.status().is_success() {
            let status = response.status();
            let resp_body = response.text().await.unwrap_or_default();
            return Err(SecretsError::OperationFailed(format!(
                "HTTP {status}: {resp_body}"
            )));
        }

        Ok(())
    }

    /// Delete a secret by key and environment.
    pub async fn delete_secret(&mut self, key: &str, env: &str) -> Result<(), SecretsError> {
        let token = self.ensure_token().await?;

        let url = format!(
            "{}/api/v1/secrets/raw/{}",
            self.config.base_url,
            key.trim_start_matches('/')
        );

        let response = self
            .http
            .delete(&url)
            .bearer_auth(&token)
            .query(&[
                ("environment", env),
                ("project_id", &self.config.project_id),
            ])
            .send()
            .await
            .map_err(SecretsError::http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SecretsError::OperationFailed(format!(
                "HTTP {status}: {body}"
            )));
        }

        Ok(())
    }
}

/// Parse the `{ "secret": { "secretKey", "secretValue" } }` payload shared
/// by the raw-secret read responses (v1 and v4 alike).
pub(super) fn parse_secret_payload(
    secret_data: &serde_json::Value,
    fallback_key: &str,
    environment: &str,
    path: Option<String>,
) -> Result<SecretValue, SecretsError> {
    let secret = secret_data
        .get("secret")
        .ok_or_else(|| SecretsError::OperationFailed("missing 'secret' in response".into()))?;

    let key = secret
        .get("secretKey")
        .and_then(|v| v.as_str())
        .unwrap_or(fallback_key)
        .to_string();

    let value = secret
        .get("secretValue")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(SecretValue {
        key,
        value,
        environment: environment.to_string(),
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_empty() {
        let config = InfisicalConfig::default();
        assert!(config.client_id.is_empty());
        assert!(config.client_secret.is_empty());
        assert!(config.project_id.is_empty());
        assert_eq!(config.base_url, DEFAULT_INFISICAL_BASE_URL);
    }

    #[test]
    fn default_config_uses_cli_documented_us_cloud_api_host() {
        let config = InfisicalConfig::default();
        assert_eq!(config.base_url, "https://app.infisical.com");
    }

    #[test]
    fn secret_value_serialization_roundtrip() {
        let sv = SecretValue {
            key: "DB_PASSWORD".into(),
            value: "s3cret".into(),
            environment: "prod".into(),
            path: Some("/database".into()),
        };

        let json = serde_json::to_string(&sv).unwrap();
        let deserialized: SecretValue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.key, "DB_PASSWORD");
        assert_eq!(deserialized.value, "s3cret");
        assert_eq!(deserialized.environment, "prod");
    }

    #[test]
    fn secret_value_optional_path() {
        let json = r#"{"key":"K","value":"V","environment":"dev"}"#;
        let sv: SecretValue = serde_json::from_str(json).unwrap();
        assert!(sv.path.is_none());
    }

    /// Official docs (observed 2026-09-27,
    /// https://infisical.com/docs/api-reference/endpoints/universal-auth/login)
    /// return `accessToken`/`expiresIn`/`tokenType`; the snake_case fixture
    /// shape no longer describes the live contract.
    #[test]
    fn token_response_matches_documented_camelcase_shape() {
        let docs_json =
            r#"{"accessToken":"tok","expiresIn":300,"accessTokenMaxTTL":600,"tokenType":"Bearer"}"#;
        let parsed: TokenResponse =
            serde_json::from_str(docs_json).expect("documented shape must parse");
        assert_eq!(parsed.access_token, "tok");
        assert_eq!(parsed.expires_in, 300);
        assert_eq!(parsed._token_type, "Bearer");
    }
}
