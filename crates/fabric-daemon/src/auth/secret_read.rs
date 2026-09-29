//! Official v4 named-secret read for [`InfisicalClient`].
//!
//! The inherent impl lives in this sibling module so `auth/secrets.rs`
//! stays focused on the legacy v1 surface. It reuses the client's
//! universal-auth token cache and HTTP pool — no auth or transport logic
//! is duplicated.

use super::secrets::{parse_secret_payload, InfisicalClient, SecretValue, SecretsError};

impl InfisicalClient {
    /// Fetch a named secret from a folder via the official v4 read
    /// endpoint (`GET /api/v4/secrets/{secretName}`, docs observed
    /// 2026-09-24, https://infisical.com/docs/api-reference/endpoints/secrets/read).
    /// Query keys follow that contract: `projectId` (required),
    /// `environment`, `secretPath`, `type`, `viewSecretValue`.
    ///
    /// # Arguments
    /// * `name` - The secret name (e.g., "WORKOS_CLIENT_SECRET")
    /// * `folder` - The secret folder/path (e.g., "/shared/workos")
    /// * `environment` - The environment slug (e.g., "dev")
    pub async fn get_secret_by_name(
        &mut self,
        name: &str,
        folder: &str,
        environment: &str,
    ) -> Result<SecretValue, SecretsError> {
        let token = self.ensure_token().await?;

        let url = format!("{}/api/v4/secrets/{}", self.config.base_url, name);

        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .query(&[
                ("projectId", self.config.project_id.as_str()),
                ("environment", environment),
                ("secretPath", folder),
                ("type", "shared"),
                ("viewSecretValue", "true"),
            ])
            .send()
            .await
            .map_err(SecretsError::http)?;

        if response.status().as_u16() == 404 {
            return Err(SecretsError::NotFound(format!(
                "secret '{name}' at folder '{folder}' in environment '{environment}'"
            )));
        }

        if !response.status().is_success() {
            // Status/category only: never read or propagate the response body,
            // so a third-party payload can never surface in logs or warnings.
            let status = response.status();
            return Err(SecretsError::OperationFailed(format!("HTTP {status}")));
        }

        let secret_data: serde_json::Value = response.json().await.map_err(SecretsError::http)?;

        parse_secret_payload(&secret_data, name, environment, Some(folder.to_string()))
    }
}
