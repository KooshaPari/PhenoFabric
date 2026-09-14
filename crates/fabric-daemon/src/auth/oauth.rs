//! WorkOS OAuth provider for fabric-daemon.
//!
//! Handles OAuth 2.0 authorization code flow with WorkOS:
//! - Generate authorization URLs
//! - Exchange authorization codes for tokens
//! - Refresh access tokens
//! - Retrieve authenticated user info

#![allow(dead_code)]

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Default WorkOS API base URL.
const DEFAULT_WORKOS_BASE_URL: &str = "https://api.workos.com";

/// Errors that can occur during OAuth operations.
#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("invalid authorization code")]
    InvalidCode,

    #[error("token exchange failed: {0}")]
    TokenExchange(String),

    #[error("token refresh failed: {0}")]
    TokenRefresh(String),

    #[error("user info fetch failed: {0}")]
    UserInfo(String),

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl OAuthError {
    /// Create an HTTP error from a reqwest error.
    fn http(err: reqwest::Error) -> Self {
        Self::Http(err.to_string())
    }

    /// Create a serialization error from a serde_json error.
    fn _serialization(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

/// Configuration for the WorkOS OAuth provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkOsConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    /// WorkOS API base URL. Defaults to `https://api.workos.com`.
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_base_url() -> String {
    DEFAULT_WORKOS_BASE_URL.to_string()
}

impl Default for WorkOsConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: String::new(),
            base_url: DEFAULT_WORKOS_BASE_URL.to_string(),
        }
    }
}

/// OAuth configuration sent to the client for initiating login.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// The WorkOS client ID.
    pub client_id: String,
    /// The redirect URI after authentication.
    pub redirect_uri: String,
    /// OAuth scopes to request.
    pub scopes: Vec<String>,
}

/// An authorization request containing the URL and state parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    /// The full authorization URL to redirect the user to.
    pub url: String,
    /// The state parameter for CSRF protection.
    pub state: String,
}

/// Response from token exchange or refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// The access token for API calls.
    pub access_token: String,
    /// The refresh token for obtaining new access tokens.
    pub refresh_token: String,
    /// Time until the access token expires (in seconds).
    pub expires_in: u64,
    /// Token type (always "Bearer").
    pub token_type: String,
    /// The authenticated WorkOS user.
    pub user: WorkOsUser,
}

/// A WorkOS-authenticated user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkOsUser {
    /// WorkOS user ID.
    pub id: String,
    /// User's email address.
    pub email: String,
    /// User's display name.
    pub name: String,
    /// Organization ID, if the user belongs to one.
    pub org_id: Option<String>,
}

/// WorkOS OAuth provider.
///
/// Handles the full OAuth 2.0 authorization code flow:
/// ```text
/// 1. generate_auth_url() -> AuthorizationRequest
/// 2. User completes login at WorkOS
/// 3. exchange_code(code) -> TokenResponse
/// 4. get_user(access_token) -> WorkOsUser
/// ```
#[derive(Debug, Clone)]
pub struct WorkOsProvider {
    config: WorkOsConfig,
    http: Client,
}

impl WorkOsProvider {
    /// Create a new WorkOS provider.
    pub fn new(config: WorkOsConfig) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to create HTTP client");
        Self { config, http }
    }

    /// Create a new WorkOS provider with a custom HTTP client (for testing).
    pub fn with_client(config: WorkOsConfig, http: Client) -> Self {
        Self { config, http }
    }

    /// Generate an authorization URL for initiating OAuth login.
    ///
    /// Returns an `AuthorizationRequest` with the full URL and a random
    /// `state` parameter for CSRF protection.
    pub fn generate_auth_url(&self) -> AuthorizationRequest {
        let state = uuid::Uuid::new_v4().to_string();
        let scopes = self.config.scopes_string();

        let url = format!(
            "{}/authorize?response_type=code&client_id={}&redirect_uri={}&state={}&scope={}",
            self.config.base_url,
            urlencoding(&self.config.client_id),
            urlencoding(&self.config.redirect_uri),
            urlencoding(&state),
            urlencoding(&scopes),
        );

        AuthorizationRequest { url, state }
    }

    /// Generate an authorization URL with specific scopes.
    pub fn generate_auth_url_with_scopes(&self, scopes: &[&str]) -> AuthorizationRequest {
        let state = uuid::Uuid::new_v4().to_string();
        let scope_str = scopes.join(" ");

        let url = format!(
            "{}/authorize?response_type=code&client_id={}&redirect_uri={}&state={}&scope={}",
            self.config.base_url,
            urlencoding(&self.config.client_id),
            urlencoding(&self.config.redirect_uri),
            urlencoding(&state),
            urlencoding(&scope_str),
        );

        AuthorizationRequest { url, state }
    }

    /// Exchange an authorization code for tokens.
    ///
    /// Calls the WorkOS `/oauth/token` endpoint with the authorization code.
    pub async fn exchange_code(&self, code: &str) -> Result<TokenResponse, OAuthError> {
        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("client_id", &self.config.client_id);
        params.insert("client_secret", &self.config.client_secret);
        params.insert("code", code);
        params.insert("redirect_uri", &self.config.redirect_uri);

        let url = format!("{}/oauth/token", self.config.base_url);

        let response = self
            .http
            .post(&url)
            .json(&params)
            .send()
            .await
            .map_err(OAuthError::http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenExchange(format!(
                "HTTP {status}: {body}"
            )));
        }

        let token_data: TokenData = response
            .json()
            .await
            .map_err(OAuthError::http)?;

        // Fetch user info with the new access token.
        let user = self.get_user(&token_data.access_token).await?;

        Ok(TokenResponse {
            access_token: token_data.access_token,
            refresh_token: token_data.refresh_token,
            expires_in: token_data.expires_in,
            token_type: token_data.token_type,
            user,
        })
    }

    /// Refresh an access token using a refresh token.
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenResponse, OAuthError> {
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("client_id", &self.config.client_id);
        params.insert("client_secret", &self.config.client_secret);
        params.insert("refresh_token", refresh_token);

        let url = format!("{}/oauth/token", self.config.base_url);

        let response = self
            .http
            .post(&url)
            .json(&params)
            .send()
            .await
            .map_err(OAuthError::http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenRefresh(format!(
                "HTTP {status}: {body}"
            )));
        }

        let token_data: TokenData = response
            .json()
            .await
            .map_err(OAuthError::http)?;

        // Fetch user info with the refreshed access token.
        let user = self.get_user(&token_data.access_token).await?;

        Ok(TokenResponse {
            access_token: token_data.access_token,
            refresh_token: token_data.refresh_token,
            expires_in: token_data.expires_in,
            token_type: token_data.token_type,
            user,
        })
    }

    /// Retrieve the authenticated user's info from WorkOS.
    pub async fn get_user(&self, access_token: &str) -> Result<WorkOsUser, OAuthError> {
        let url = format!("{}/users/me", self.config.base_url);

        let response = self
            .http
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(OAuthError::http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OAuthError::UserInfo(format!(
                "HTTP {status}: {body}"
            )));
        }

        let user: WorkOsUser = response
            .json()
            .await
            .map_err(OAuthError::http)?;

        Ok(user)
    }

    /// Introspect a token to check if it is active.
    ///
    /// Calls the WorkOS `/oauth/token/introspect` endpoint.
    pub async fn introspect_token(&self, token: &str) -> Result<TokenIntrospection, OAuthError> {
        let mut params = HashMap::new();
        params.insert("token", token);
        params.insert("client_id", &self.config.client_id);
        params.insert("client_secret", &self.config.client_secret);

        let url = format!("{}/oauth/token/introspect", self.config.base_url);

        let response = self
            .http
            .post(&url)
            .json(&params)
            .send()
            .await
            .map_err(OAuthError::http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenExchange(format!(
                "HTTP {status}: {body}"
            )));
        }

        let introspection: TokenIntrospection = response
            .json()
            .await
            .map_err(OAuthError::http)?;

        Ok(introspection)
    }
}

// ---------------------------------------------------------------------------
// Internal types for WorkOS API responses
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TokenData {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    token_type: String,
}

/// Result of token introspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenIntrospection {
    /// Whether the token is currently active.
    pub active: bool,
    /// The client ID the token was issued to.
    #[serde(default)]
    pub client_id: Option<String>,
    /// The scope of the token.
    #[serde(default)]
    pub scope: Option<String>,
    /// The subject (user ID) of the token.
    #[serde(default)]
    pub sub: Option<String>,
    /// The token expiration timestamp (Unix epoch).
    #[serde(default)]
    pub exp: Option<u64>,
    /// The token issuance timestamp (Unix epoch).
    #[serde(default)]
    pub iat: Option<u64>,
    /// The token type.
    #[serde(default)]
    pub token_type: Option<String>,
}

impl WorkOsConfig {
    /// Get scopes as a space-separated string.
    fn scopes_string(&self) -> String {
        // Default scopes for WorkOS OAuth.
        "openid email profile".to_string()
    }
}

/// URL-encode a string for use in query parameters.
fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_auth_url_produces_valid_url() {
        let config = WorkOsConfig {
            client_id: "test_client_id".into(),
            client_secret: "secret".into(),
            redirect_uri: "http://localhost:9400/callback".into(),
            base_url: DEFAULT_WORKOS_BASE_URL.to_string(),
        };

        let provider = WorkOsProvider::new(config);
        let auth_req = provider.generate_auth_url();

        assert!(auth_req.url.contains("client_id=test_client_id"));
        assert!(auth_req.url.contains("response_type=code"));
        assert!(auth_req.url.contains("redirect_uri="));
        assert!(!auth_req.state.is_empty());
    }

    #[test]
    fn generate_auth_url_with_custom_scopes() {
        let config = WorkOsConfig::default();
        let provider = WorkOsProvider::new(config);
        let auth_req = provider.generate_auth_url_with_scopes(&["openid", "email"]);

        assert!(auth_req.url.contains("scope=openid%20email"));
    }

    #[test]
    fn urlencoding_works() {
        assert_eq!(urlencoding("hello"), "hello");
        assert_eq!(urlencoding("a b"), "a%20b");
        assert_eq!(urlencoding("a+b"), "a%2Bb");
    }

    #[test]
    fn token_response_serialization_roundtrip() {
        let resp = TokenResponse {
            access_token: "at_123".into(),
            refresh_token: "rt_456".into(),
            expires_in: 3600,
            token_type: "Bearer".into(),
            user: WorkOsUser {
                id: "user_1".into(),
                email: "test@example.com".into(),
                name: "Test User".into(),
                org_id: Some("org_1".into()),
            },
        };

        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: TokenResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.access_token, "at_123");
        assert_eq!(deserialized.user.email, "test@example.com");
    }

    #[test]
    fn workos_user_serialization_roundtrip() {
        let user = WorkOsUser {
            id: "user_1".into(),
            email: "test@example.com".into(),
            name: "Test User".into(),
            org_id: None,
        };

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: WorkOsUser = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "user_1");
        assert!(deserialized.org_id.is_none());
    }
}
