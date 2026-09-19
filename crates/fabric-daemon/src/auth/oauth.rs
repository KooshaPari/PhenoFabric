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

    #[error("magic auth failed: {0}")]
    MagicAuth(String),

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

/// Grant type for Magic Auth (passwordless) authentication.
pub const MAGIC_AUTH_GRANT_TYPE: &str = "urn:workos:oauth:grant-type:magic-auth:code";

/// A WorkOS Magic Auth code (passwordless one-time code).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicAuthCode {
    /// Magic Auth code ID.
    pub id: String,
    /// The user the code was created for.
    pub user_id: String,
    /// The email address the code was sent to.
    pub email: String,
    /// When the code expires.
    pub expires_at: String,
    /// When the code was created.
    pub created_at: String,
}

/// WorkOS AuthKit provider.
///
/// Handles the AuthKit authorization code flow, PKCE-capable:
/// ```text
/// 1. generate_auth_url() -> AuthorizationRequest
/// 2. User completes login at WorkOS
/// 3. authenticate_with_authorization_code(code, code_verifier) -> TokenResponse
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

    /// Exchange an AuthKit authorization code for tokens.
    ///
    /// AuthKit codes come from `/user_management/authorize` and must be
    /// exchanged at `/user_management/authenticate`. The `/oauth/token`
    /// endpoint belongs to plain WorkOS OAuth apps and rejects AuthKit codes,
    /// which is why this posts to the user-management endpoint instead.
    ///
    /// Public clients (desktop apps) send a PKCE `code_verifier` and no client
    /// secret; confidential clients send `client_secret` instead. The endpoint
    /// accepts either, so the verifier decides which credential is sent
    /// (verified against the WorkOS API reference, 2026-09-19).
    pub async fn authenticate_with_authorization_code(
        &self,
        code: &str,
        code_verifier: Option<&str>,
    ) -> Result<TokenResponse, OAuthError> {
        let params = authorization_code_params(
            &self.config.client_id,
            &self.config.client_secret,
            code,
            code_verifier,
        );

        let url = format!("{}/user_management/authenticate", self.config.base_url);

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
            return Err(OAuthError::TokenExchange(format!("HTTP {status}: {body}")));
        }

        let data: AuthenticateData = response.json().await.map_err(OAuthError::http)?;

        Ok(data.into_token_response())
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
            return Err(OAuthError::TokenRefresh(format!("HTTP {status}: {body}")));
        }

        let token_data: TokenData = response.json().await.map_err(OAuthError::http)?;

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

    /// Send a Magic Auth (passwordless) code to the user's email address.
    ///
    /// Calls the WorkOS `POST /user_management/magic_auth` endpoint, which
    /// creates a one-time 6-digit code (10 minute expiry) and emails it.
    /// The user then completes login via `authenticate_with_magic_auth_code`.
    pub async fn send_magic_auth_code(&self, email: &str) -> Result<MagicAuthCode, OAuthError> {
        let mut params = HashMap::new();
        params.insert("email", email);

        let url = format!("{}/user_management/magic_auth", self.config.base_url);

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
            return Err(OAuthError::MagicAuth(format!("HTTP {status}: {body}")));
        }

        let magic_auth: MagicAuthCode = response.json().await.map_err(OAuthError::http)?;

        Ok(magic_auth)
    }

    /// Authenticate a user with a Magic Auth code (passwordless).
    ///
    /// Calls the WorkOS `POST /user_management/authenticate` endpoint with the
    /// `urn:workos:oauth:grant-type:magic-auth:code` grant type.
    pub async fn authenticate_with_magic_auth_code(
        &self,
        email: &str,
        code: &str,
    ) -> Result<TokenResponse, OAuthError> {
        let mut params = HashMap::new();
        params.insert("grant_type", MAGIC_AUTH_GRANT_TYPE);
        params.insert("client_id", &self.config.client_id);
        params.insert("client_secret", &self.config.client_secret);
        params.insert("email", email);
        params.insert("code", code);

        let url = format!("{}/user_management/authenticate", self.config.base_url);

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
            return Err(OAuthError::MagicAuth(format!("HTTP {status}: {body}")));
        }

        let token_data: TokenData = response.json().await.map_err(OAuthError::http)?;

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
            return Err(OAuthError::UserInfo(format!("HTTP {status}: {body}")));
        }

        let user: WorkOsUser = response.json().await.map_err(OAuthError::http)?;

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
            return Err(OAuthError::TokenExchange(format!("HTTP {status}: {body}")));
        }

        let introspection: TokenIntrospection = response.json().await.map_err(OAuthError::http)?;

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

/// Grant type for the AuthKit authorization-code exchange.
pub const AUTHORIZATION_CODE_GRANT_TYPE: &str = "authorization_code";

/// Fallback access-token lifetime, used when WorkOS omits `expires_in` and the
/// token carries no readable `exp` claim.
const DEFAULT_ACCESS_TOKEN_TTL_SECS: u64 = 3600;

/// Build the parameters for the AuthKit authorization-code exchange.
///
/// A PKCE `code_verifier` marks a public client, which must not send a client
/// secret; without one the client is confidential and authenticates with the
/// secret. Sending both is rejected, so this is strictly either/or.
fn authorization_code_params<'a>(
    client_id: &'a str,
    client_secret: &'a str,
    code: &'a str,
    code_verifier: Option<&'a str>,
) -> HashMap<&'a str, &'a str> {
    let mut params = HashMap::new();
    params.insert("grant_type", AUTHORIZATION_CODE_GRANT_TYPE);
    params.insert("client_id", client_id);
    params.insert("code", code);
    match code_verifier {
        Some(verifier) => {
            params.insert("code_verifier", verifier);
        }
        None if !client_secret.is_empty() => {
            params.insert("client_secret", client_secret);
        }
        None => {}
    }
    params
}

/// User object as returned by `/user_management/authenticate`.
///
/// WorkOS returns `first_name`/`last_name` here, unlike the `name` field this
/// crate's `WorkOsUser` exposes, so the two are mapped explicitly.
#[derive(Debug, Clone, Deserialize)]
struct AuthKitUser {
    id: String,
    email: String,
    #[serde(default)]
    first_name: Option<String>,
    #[serde(default)]
    last_name: Option<String>,
}

/// Response from `/user_management/authenticate`.
#[derive(Debug, Clone, Deserialize)]
struct AuthenticateData {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    token_type: Option<String>,
    user: AuthKitUser,
    #[serde(default)]
    organization_id: Option<String>,
}

impl AuthenticateData {
    /// Map the AuthKit response onto this crate's `TokenResponse`.
    ///
    /// The endpoint does not always return `expires_in`, so the access token's
    /// `exp` claim is the fallback. The token is only read, never trusted: it
    /// was received over TLS from WorkOS moments earlier and is not used to
    /// authorize anything here.
    fn into_token_response(self) -> TokenResponse {
        let expires_in = self
            .expires_in
            .or_else(|| access_token_expires_in(&self.access_token))
            .unwrap_or(DEFAULT_ACCESS_TOKEN_TTL_SECS);

        let name = match (self.user.first_name, self.user.last_name) {
            (Some(first), Some(last)) => format!("{first} {last}"),
            (Some(first), None) => first,
            (None, Some(last)) => last,
            (None, None) => self.user.email.clone(),
        };

        TokenResponse {
            access_token: self.access_token,
            refresh_token: self.refresh_token.unwrap_or_default(),
            expires_in,
            token_type: self.token_type.unwrap_or_else(|| "Bearer".into()),
            user: WorkOsUser {
                id: self.user.id,
                email: self.user.email,
                name,
                org_id: self.organization_id,
            },
        }
    }
}

/// Seconds until the access token's `exp` claim, if it has a readable one.
fn access_token_expires_in(access_token: &str) -> Option<u64> {
    #[derive(Deserialize)]
    struct Claims {
        exp: u64,
    }

    let decoded = jsonwebtoken::dangerous::insecure_decode::<Claims>(access_token).ok()?;
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    Some(decoded.claims.exp.saturating_sub(now))
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

    #[test]
    fn pkce_verifier_replaces_client_secret() {
        let params =
            authorization_code_params("client_1", "sk_secret", "code_1", Some("verifier_1"));
        assert_eq!(params.get("grant_type"), Some(&"authorization_code"));
        assert_eq!(params.get("client_id"), Some(&"client_1"));
        assert_eq!(params.get("code"), Some(&"code_1"));
        assert_eq!(params.get("code_verifier"), Some(&"verifier_1"));
        assert!(
            !params.contains_key("client_secret"),
            "a public client must not send a client secret"
        );
    }

    #[test]
    fn confidential_client_sends_secret_without_verifier() {
        let params = authorization_code_params("client_1", "sk_secret", "code_1", None);
        assert_eq!(params.get("client_secret"), Some(&"sk_secret"));
        assert!(!params.contains_key("code_verifier"));
    }

    #[test]
    fn missing_secret_and_verifier_still_builds_request() {
        // A misconfigured daemon must produce a clean API error, not a panic or
        // a malformed request.
        let params = authorization_code_params("client_1", "", "code_1", None);
        assert_eq!(params.get("client_id"), Some(&"client_1"));
        assert!(!params.contains_key("client_secret"));
        assert!(!params.contains_key("code_verifier"));
    }

    #[test]
    fn authkit_response_maps_name_and_organization() {
        let data = AuthenticateData {
            access_token: "not-a-jwt".into(),
            refresh_token: Some("rt_1".into()),
            expires_in: Some(1800),
            token_type: None,
            user: AuthKitUser {
                id: "user_1".into(),
                email: "dev@example.com".into(),
                first_name: Some("Dev".into()),
                last_name: Some("User".into()),
            },
            organization_id: Some("org_1".into()),
        };

        let tokens = data.into_token_response();
        assert_eq!(tokens.user.name, "Dev User");
        assert_eq!(tokens.user.org_id.as_deref(), Some("org_1"));
        assert_eq!(tokens.expires_in, 1800);
        // WorkOS omits token_type on this endpoint; Bearer is the only kind
        // AuthKit issues.
        assert_eq!(tokens.token_type, "Bearer");
        assert_eq!(tokens.refresh_token, "rt_1");
    }

    #[test]
    fn authkit_response_falls_back_to_default_ttl() {
        // No expires_in and a non-JWT access token: the fallback must still be
        // a positive lifetime, never zero, which would report the session as
        // instantly expired.
        let data = AuthenticateData {
            access_token: "not-a-jwt".into(),
            refresh_token: None,
            expires_in: None,
            token_type: None,
            user: AuthKitUser {
                id: "user_1".into(),
                email: "dev@example.com".into(),
                first_name: None,
                last_name: None,
            },
            organization_id: None,
        };

        let tokens = data.into_token_response();
        assert_eq!(tokens.expires_in, DEFAULT_ACCESS_TOKEN_TTL_SECS);
        // With no names, the email is the only sensible display name.
        assert_eq!(tokens.user.name, "dev@example.com");
        assert_eq!(tokens.refresh_token, "");
    }

    #[test]
    fn expires_in_read_from_jwt_exp_claim() {
        // A real AuthKit access token is a JWT; when the response omits
        // expires_in, the exp claim is the fallback.
        #[derive(serde::Serialize)]
        struct Claims {
            exp: u64,
            sub: &'static str,
        }

        let exp = (chrono::Utc::now().timestamp() + 900) as u64;
        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &Claims { exp, sub: "user_1" },
            &jsonwebtoken::EncodingKey::from_secret(b"test-secret"),
        )
        .unwrap();

        let remaining = access_token_expires_in(&token).expect("exp claim readable");
        assert!(
            remaining > 800 && remaining <= 900,
            "unexpected remaining lifetime: {remaining}"
        );
    }

    #[test]
    fn non_jwt_access_token_has_no_readable_expiry() {
        assert!(access_token_expires_in("not-a-jwt").is_none());
    }
}
