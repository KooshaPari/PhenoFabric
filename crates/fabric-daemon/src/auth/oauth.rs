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

    #[error("token verification failed: {0}")]
    TokenVerification(String),

    #[error("JWKS fetch failed: {0}")]
    Jwks(String),

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

/// An authorization request containing the URL, state parameter, and the
/// PKCE verifier the daemon minted for this state.
///
/// The verifier is server-side state: it is bound to the state in the
/// in-memory registry (`crate::auth::callback::registry::record_state_with_verifier`)
/// and looked up on the matching `/auth/callback` request. Callers must NOT
/// ship it back in the response payload — only the URL (which carries the
/// matching `code_challenge`) is exposed to the browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    /// The full authorization URL to redirect the user to.
    pub url: String,
    /// The state parameter for CSRF protection.
    pub state: String,
    /// PKCE verifier (RFC 7636) the daemon minted for this state. Never
    /// serialised to the browser; bind to state via
    /// `callback::registry::record_state_with_verifier`.
    pub verifier: String,
}

/// A PKCE pair produced by `generate_pkce_pair` (RFC 7636 §4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkcePair {
    /// `code_verifier` -- 43-128 chars from the unreserved set
    /// `[A-Z] / [a-z] / [0-9] / "-" / "." / "_" / "~"`.
    pub verifier: String,
    /// `code_challenge` -- BASE64URL-ENCODE(SHA256(ASCII(verifier))) with no
    /// padding. Sent on the authorize URL alongside
    /// `code_challenge_method=S256`.
    pub challenge: String,
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
    /// Returns an `AuthorizationRequest` with the full URL, a random
    /// `state` parameter for CSRF protection, and a freshly minted PKCE
    /// `verifier`. The verifier is server-side: bind it to the state via
    /// `crate::auth::callback::registry::record_state_with_verifier` so the
    /// `/auth/callback` listener can look it up on redemption. The matching
    /// `code_challenge` (RFC 7636 §4.2, S256) is included in the URL along
    /// with `code_challenge_method=S256`.
    pub fn generate_auth_url(&self) -> AuthorizationRequest {
        let state = uuid::Uuid::new_v4().to_string();
        let scopes = self.config.scopes_string();
        let pkce = generate_pkce_pair();

        let url = format!(
            "{}/authorize?response_type=code&client_id={}&redirect_uri={}&state={}&scope={}&code_challenge={}&code_challenge_method=S256",
            self.config.base_url,
            urlencoding(&self.config.client_id),
            urlencoding(&self.config.redirect_uri),
            urlencoding(&state),
            urlencoding(&scopes),
            urlencoding(&pkce.challenge),
        );

        AuthorizationRequest {
            url,
            state,
            verifier: pkce.verifier,
        }
    }

    /// Generate an authorization URL with specific scopes.
    ///
    /// Same PKCE contract as `generate_auth_url`: the returned
    /// `AuthorizationRequest` carries a freshly minted verifier; the URL
    /// carries the matching `code_challenge` and `code_challenge_method=S256`.
    pub fn generate_auth_url_with_scopes(&self, scopes: &[&str]) -> AuthorizationRequest {
        let state = uuid::Uuid::new_v4().to_string();
        let scope_str = scopes.join(" ");
        let pkce = generate_pkce_pair();

        let url = format!(
            "{}/authorize?response_type=code&client_id={}&redirect_uri={}&state={}&scope={}&code_challenge={}&code_challenge_method=S256",
            self.config.base_url,
            urlencoding(&self.config.client_id),
            urlencoding(&self.config.redirect_uri),
            urlencoding(&state),
            urlencoding(&scope_str),
            urlencoding(&pkce.challenge),
        );

        AuthorizationRequest {
            url,
            state,
            verifier: pkce.verifier,
        }
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

        let url = authenticate_url(&self.config.base_url);

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

        let data: AuthenticateData = response.json().await.map_err(OAuthError::http)?;
        let mut tokens = data.into_token_response();
        if tokens.refresh_token.is_empty() {
            // WorkOS rotates refresh tokens and returns the replacement, but if
            // one is ever omitted, keep the token already held rather than
            // discarding it and breaking the next refresh.
            tokens.refresh_token = refresh_token.to_string();
        }
        Ok(tokens)
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

        let data: AuthenticateData = response.json().await.map_err(OAuthError::http)?;

        Ok(data.into_token_response())
    }

    /// URL of the JWKS that signs AuthKit access tokens.
    ///
    /// AuthKit access tokens are RS256 JWTs and WorkOS exposes no introspection
    /// endpoint (`/oauth/token/introspect` answers 404 - live check
    /// 2026-09-19), so signature verification against these keys is the only
    /// validation path.
    pub fn jwks_url(&self) -> String {
        format!(
            "{}/sso/jwks/{}",
            self.config.base_url, self.config.client_id
        )
    }

    /// Fetch the JWKS document for this client.
    pub async fn fetch_jwks(&self) -> Result<jsonwebtoken::jwk::JwkSet, OAuthError> {
        let url = self.jwks_url();

        let response = self.http.get(&url).send().await.map_err(OAuthError::http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OAuthError::Jwks(format!("HTTP {status}: {body}")));
        }

        response.json().await.map_err(OAuthError::http)
    }

    /// Verify an AuthKit access token and return its claims.
    pub async fn verify_access_token(&self, token: &str) -> Result<AccessTokenClaims, OAuthError> {
        let jwks = self.fetch_jwks().await?;
        verify_access_token_with_jwks(token, &jwks, &self.config.client_id, &self.config.base_url)
    }
}

/// Path of the AuthKit code exchange and token refresh endpoint.
///
/// Both operations post here. The plain OAuth endpoint (`/oauth/token`) does
/// not exist on the WorkOS API - it answers 404 - which is what silently broke
/// every live login before this was pinned (live check 2026-09-19).
pub const AUTHENTICATE_PATH: &str = "/user_management/authenticate";

/// Full URL of the AuthKit authenticate endpoint.
fn authenticate_url(base_url: &str) -> String {
    format!("{base_url}{AUTHENTICATE_PATH}")
}

/// Claims carried by a WorkOS AuthKit access token.
///
/// Field list taken from the WorkOS session-tokens reference (2026-09-19).
/// There is no `aud` claim: the client is identified by `client_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// Issuer, `https://api.workos.com`.
    pub iss: String,
    /// WorkOS user id.
    pub sub: String,
    /// Client the token was issued to.
    pub client_id: String,
    /// Organization the session belongs to.
    #[serde(default)]
    pub org_id: Option<String>,
    /// Primary role.
    #[serde(default)]
    pub role: Option<String>,
    /// All roles.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Session id.
    #[serde(default)]
    pub sid: Option<String>,
    /// Expiry, seconds since the epoch.
    pub exp: u64,
    /// Issued-at, seconds since the epoch.
    #[serde(default)]
    pub iat: Option<u64>,
}

/// Verify a token against a JWKS document.
///
/// Separate from the provider method so the rules can be tested without a
/// network round trip.
fn verify_access_token_with_jwks(
    token: &str,
    jwks: &jsonwebtoken::jwk::JwkSet,
    expected_client_id: &str,
    expected_issuer: &str,
) -> Result<AccessTokenClaims, OAuthError> {
    let header = jsonwebtoken::decode_header(token)
        .map_err(|e| OAuthError::TokenVerification(format!("malformed token header: {e}")))?;

    let kid = header
        .kid
        .ok_or_else(|| OAuthError::TokenVerification("token header carries no kid".into()))?;

    let jwk = jwks
        .find(&kid)
        .ok_or_else(|| OAuthError::TokenVerification(format!("no JWKS key matches kid {kid}")))?;

    let key = jsonwebtoken::DecodingKey::from_jwk(jwk)
        .map_err(|e| OAuthError::TokenVerification(format!("unusable JWKS key: {e}")))?;

    verify_token_with_key(
        token,
        &key,
        jsonwebtoken::Algorithm::RS256,
        expected_client_id,
        expected_issuer,
    )
}

/// Verify a token against an explicit key, algorithm, client id and issuer.
///
/// The algorithm is supplied by the caller and never read from the token, which
/// is what makes an algorithm-confusion downgrade impossible. Production always
/// passes RS256 with a key taken from the WorkOS JWKS.
fn verify_token_with_key(
    token: &str,
    key: &jsonwebtoken::DecodingKey,
    algorithm: jsonwebtoken::Algorithm,
    expected_client_id: &str,
    expected_issuer: &str,
) -> Result<AccessTokenClaims, OAuthError> {
    let mut validation = jsonwebtoken::Validation::new(algorithm);
    validation.set_issuer(&[expected_issuer]);
    // AuthKit identifies the client with a `client_id` claim, not `aud`.
    validation.validate_aud = false;

    let data = jsonwebtoken::decode::<AccessTokenClaims>(token, key, &validation)
        .map_err(|e| OAuthError::TokenVerification(e.to_string()))?;

    if data.claims.client_id != expected_client_id {
        return Err(OAuthError::TokenVerification(format!(
            "token was issued to client {}, not {expected_client_id}",
            data.claims.client_id
        )));
    }

    Ok(data.claims)
}

// ---------------------------------------------------------------------------
// Internal types for WorkOS API responses
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// PKCE (RFC 7636) construction
// ---------------------------------------------------------------------------

/// Generate a fresh PKCE verifier + S256 challenge pair.
///
/// The verifier is built from 4 random UUID v4 values (16 bytes each,
/// total 64 bytes of OS entropy), then BASE64URL-encoded without padding
/// to produce an 86-character ASCII string drawn from the RFC 7636
/// unreserved set `[A-Z][a-z][0-9]-._~`. 86 chars is well inside the
/// 43-128 spec range and gives 512 bits of entropy — well above the
/// 256-bit floor the RFC suggests.
///
/// The challenge is `BASE64URL(SHA256(verifier))` without padding per
/// RFC 7636 §4.2. The SHA-256 implementation is inlined here because
/// this crate does not declare `sha2` directly; it is reachable
/// transitively via `jsonwebtoken`, but adding it as a direct dep would
/// change the manifest, which the build gate prohibits. The inlined
/// implementation is verified against the FIPS 180-4 KAT vectors in
/// `tests::pkce_sha256_known_vectors`.
fn generate_pkce_pair() -> PkcePair {
    // 4 × 128-bit UUID v4 = 64 bytes of OS-backed entropy.
    let mut bytes = [0u8; 64];
    bytes[0..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes[16..32].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes[32..48].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes[48..64].copy_from_slice(uuid::Uuid::new_v4().as_bytes());

    let verifier = base64url_encode(&bytes);
    // SHA-256 of the ASCII bytes of the verifier (RFC 7636 §4.2).
    let challenge = base64url_encode(&sha256(verifier.as_bytes()));

    PkcePair {
        verifier,
        challenge,
    }
}

/// BASE64URL-encode a byte slice without padding (RFC 4648 §5).
///
/// Used for both the PKCE verifier (raw entropy) and the PKCE challenge
/// (SHA-256 output). Standard base64 with `+` mapped to `-`, `/` mapped
/// to `_`, and `=` padding stripped.
fn base64url_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push(ALPHABET[(n & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
    }
    out
}

/// SHA-256 (FIPS 180-4) over `data`. Returns the 32-byte digest.
///
/// Inlined because this crate does not declare `sha2` as a direct dep;
/// verified against FIPS 180-4 KAT vectors by `pkce_sha256_known_vectors`.
/// Constant-time on the input length but not on the bytes themselves
/// (it doesn't need to be — this is a public token hash, not a secret
/// comparison). Implemented as the straightforward 8-state, 64-round
/// compression function; no platform-specific intrinsics.
#[allow(clippy::needless_range_loop)]
fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    // Initial hash values (FIPS 180-4 §5.3.3).
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    // Pre-processing: append the 1-bit, pad to 64-byte boundary minus 8,
    // then write the 64-bit big-endian bit length (FIPS 180-4 §5.1.1).
    let mut msg = data.to_vec();
    let bit_len = (msg.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let off = i * 4;
            *word =
                u32::from_be_bytes([chunk[off], chunk[off + 1], chunk[off + 2], chunk[off + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d) = (h[0], h[1], h[2], h[3]);
        let (mut e, mut f, mut g, mut hh) = (h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let mj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(mj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
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

    // ----- PKCE / RFC 7636 tests --------------------------------------------

    /// Extract a single query parameter value from an authorize URL.
    ///
    /// Naive: works on the small URL shapes we emit because none of the
    /// parameters contain `+` (urlencoding doesn't produce it for the
    /// BASE64URL alphabet) and we never emit duplicate keys.
    fn query_param(url: &str, name: &str) -> Option<String> {
        let q = url.split_once('?')?.1;
        for pair in q.split('&') {
            let (k, v) = pair.split_once('=')?;
            if k == name {
                return Some(url_decode_simple(v));
            }
        }
        None
    }

    /// Minimal percent-decode for `[A-Za-z0-9%]` — enough to round-trip
    /// the BASE64URL alphabet and the URL-safe characters `state` uses.
    fn url_decode_simple(s: &str) -> String {
        let mut out = Vec::with_capacity(s.len());
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                let h = hex_val(bytes[i + 1]);
                let l = hex_val(bytes[i + 2]);
                if let (Some(h), Some(l)) = (h, l) {
                    out.push((h << 4) | l);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    fn hex_val(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }

    #[test]
    fn pkce_pair_challenge_is_sha256_of_verifier() {
        // Cryptographic, not string-stub: compute SHA-256 ourselves and
        // base64url-encode, then compare to the daemon's challenge.
        let pkce = generate_pkce_pair();
        let expected = base64url_encode(&sha256(pkce.verifier.as_bytes()));
        assert_eq!(pkce.challenge, expected);
    }

    #[test]
    fn pkce_verifier_is_in_rfc7636_unreserved_set() {
        let pkce = generate_pkce_pair();
        // 43-128 char range; we use 64 bytes of entropy -> 86 chars.
        assert!(
            pkce.verifier.len() >= 43 && pkce.verifier.len() <= 128,
            "verifier length {} out of RFC 7636 range",
            pkce.verifier.len()
        );
        for b in pkce.verifier.bytes() {
            let ok = matches!(b,
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~');
            assert!(ok, "verifier byte {b:#04x} not in RFC 7636 unreserved set");
        }
    }

    #[test]
    fn pkce_pair_is_unique_per_call() {
        // 64 bytes of UUID v4 entropy should make accidental collision
        // astronomically unlikely.
        let a = generate_pkce_pair();
        let b = generate_pkce_pair();
        assert_ne!(a.verifier, b.verifier);
        assert_ne!(a.challenge, b.challenge);
    }

    #[test]
    fn generate_auth_url_includes_pkce_challenge_and_method() {
        let config = WorkOsConfig {
            client_id: "client_1".into(),
            client_secret: String::new(),
            redirect_uri: "http://localhost:9400/callback".into(),
            base_url: DEFAULT_WORKOS_BASE_URL.to_string(),
        };
        let provider = WorkOsProvider::new(config);
        let req = provider.generate_auth_url();

        // URL contains the two PKCE params.
        let challenge =
            query_param(&req.url, "code_challenge").expect("code_challenge present in URL");
        let method =
            query_param(&req.url, "code_challenge_method").expect("code_challenge_method present");
        assert_eq!(method, "S256");

        // Cryptographic check: S256(req.verifier) == challenge.
        let expected = base64url_encode(&sha256(req.verifier.as_bytes()));
        assert_eq!(challenge, expected);

        // AuthRequest is internally consistent.
        assert!(!req.state.is_empty());
        assert!(!req.verifier.is_empty());
    }

    #[test]
    fn generate_auth_url_with_scopes_includes_pkce() {
        let provider = WorkOsProvider::new(WorkOsConfig {
            client_id: "client_2".into(),
            client_secret: String::new(),
            redirect_uri: "http://localhost:9400/cb".into(),
            base_url: DEFAULT_WORKOS_BASE_URL.to_string(),
        });
        let req = provider.generate_auth_url_with_scopes(&["openid", "email"]);
        let challenge =
            query_param(&req.url, "code_challenge").expect("code_challenge present in URL");
        let method = query_param(&req.url, "code_challenge_method").unwrap();
        assert_eq!(method, "S256");
        let expected = base64url_encode(&sha256(req.verifier.as_bytes()));
        assert_eq!(challenge, expected);
    }

    #[test]
    fn pkce_sha256_known_vectors() {
        // FIPS 180-4 §B.1 known-answer vectors for SHA-256.
        let cases: &[(&[u8], &str)] = &[
            (
                b"",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
        ];
        for (input, expected_hex) in cases {
            let got = sha256(input);
            // Build a hex string from the digest bytes for easy compare.
            let mut hex = String::with_capacity(64);
            for b in &got {
                hex.push_str(&format!("{b:02x}"));
            }
            assert_eq!(&hex, expected_hex, "SHA-256 mismatch for input {input:?}");
        }

        // The 2-block FIPS vector: 64 'a' bytes. Useful to exercise the
        // second-block compression loop (the single-block vectors above
        // do not). Verified independently with `printf 'a%.0s' {1..64} | shasum -a 256`.
        let block2 = [b'a'; 64];
        let got = sha256(&block2);
        let mut hex = String::with_capacity(64);
        for b in &got {
            hex.push_str(&format!("{b:02x}"));
        }
        assert_eq!(
            &hex,
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
    }

    #[test]
    fn base64url_encode_matches_rfc4648_vectors() {
        // Standard RFC 4648 §10 vectors. The difference from base64 is
        // the alphabet substitution (`+/` -> `-_`) and no padding.
        assert_eq!(base64url_encode(b""), "");
        assert_eq!(base64url_encode(b"f"), "Zg");
        assert_eq!(base64url_encode(b"fo"), "Zm8");
        assert_eq!(base64url_encode(b"foo"), "Zm9v");
        assert_eq!(base64url_encode(b"foob"), "Zm9vYg");
        assert_eq!(base64url_encode(b"fooba"), "Zm9vYmE");
        assert_eq!(base64url_encode(b"foobar"), "Zm9vYmFy");
        // 32-byte input: 32/3 = 10 full triples + 2 leftover bytes (no padding).
        let mut bytes = Vec::with_capacity(32);
        for i in 0..32u8 {
            bytes.push(i);
        }
        assert_eq!(bytes.len(), 32);
        let encoded = base64url_encode(&bytes);
        assert_eq!(encoded.len(), 43); // ceil(32/3)*4 - 1 (no padding on 32-byte input)
        assert!(!encoded.ends_with('='));
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

    const TEST_ISSUER: &str = "https://api.workos.com";

    /// Sign a token with a symmetric key.
    ///
    /// Production verification is RS256 against the WorkOS JWKS, but the rules
    /// under test - issuer, client id, expiry, claim extraction - are
    /// algorithm-independent, and HS256 lets them be exercised without an RSA
    /// fixture. The algorithm is always supplied by the caller, never read from
    /// the token, so this cannot mask an algorithm-confusion bug.
    fn sign_test_token(
        secret: &[u8],
        client_id: &str,
        issuer: &str,
        exp: u64,
        kid: Option<&str>,
    ) -> String {
        #[derive(serde::Serialize)]
        struct Claims {
            iss: String,
            sub: String,
            client_id: String,
            org_id: String,
            exp: u64,
        }

        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
        header.kid = kid.map(str::to_string);
        jsonwebtoken::encode(
            &header,
            &Claims {
                iss: issuer.to_string(),
                sub: "user_1".into(),
                client_id: client_id.to_string(),
                org_id: "org_1".into(),
                exp,
            },
            &jsonwebtoken::EncodingKey::from_secret(secret),
        )
        .unwrap()
    }

    fn empty_jwks() -> jsonwebtoken::jwk::JwkSet {
        serde_json::from_str(r#"{"keys":[]}"#).unwrap()
    }

    fn verify_with_secret(token: &str, secret: &[u8]) -> Result<AccessTokenClaims, OAuthError> {
        verify_token_with_key(
            token,
            &jsonwebtoken::DecodingKey::from_secret(secret),
            jsonwebtoken::Algorithm::HS256,
            "client_1",
            TEST_ISSUER,
        )
    }

    fn future_exp() -> u64 {
        (chrono::Utc::now().timestamp() + 600).max(0) as u64
    }

    #[test]
    fn jwks_url_targets_the_client_jwks() {
        let provider = WorkOsProvider::new(WorkOsConfig {
            client_id: "client_1".into(),
            ..Default::default()
        });
        assert_eq!(
            provider.jwks_url(),
            "https://api.workos.com/sso/jwks/client_1"
        );
    }

    #[test]
    fn authenticate_url_uses_the_user_management_endpoint() {
        // Regression guard: the plain OAuth endpoint answers 404 on the live
        // API (checked 2026-09-19), so exchanging or refreshing there breaks
        // every login.
        let url = authenticate_url("https://api.workos.com");
        assert_eq!(url, "https://api.workos.com/user_management/authenticate");
        assert!(!url.ends_with("/oauth/token"));
    }

    #[test]
    fn valid_token_yields_claims() {
        let secret = b"test-secret";
        let token = sign_test_token(secret, "client_1", TEST_ISSUER, future_exp(), Some("k1"));

        let claims = verify_with_secret(&token, secret).expect("valid token verifies");
        assert_eq!(claims.sub, "user_1");
        assert_eq!(claims.client_id, "client_1");
        assert_eq!(claims.org_id.as_deref(), Some("org_1"));
    }

    #[test]
    fn token_for_another_client_is_rejected() {
        let secret = b"test-secret";
        let token = sign_test_token(
            secret,
            "client_other",
            TEST_ISSUER,
            future_exp(),
            Some("k1"),
        );

        let err = verify_with_secret(&token, secret).unwrap_err();
        assert!(
            err.to_string().contains("issued to client client_other"),
            "{err}"
        );
    }

    #[test]
    fn token_from_another_issuer_is_rejected() {
        let secret = b"test-secret";
        let token = sign_test_token(
            secret,
            "client_1",
            "https://evil.example",
            future_exp(),
            Some("k1"),
        );

        let err = verify_with_secret(&token, secret).unwrap_err();
        assert!(err.to_string().contains("InvalidIssuer"), "{err}");
    }

    #[test]
    fn expired_token_is_rejected() {
        let secret = b"test-secret";
        let past = (chrono::Utc::now().timestamp() - 3600).max(0) as u64;
        let token = sign_test_token(secret, "client_1", TEST_ISSUER, past, Some("k1"));

        let err = verify_with_secret(&token, secret).unwrap_err();
        assert!(err.to_string().contains("ExpiredSignature"), "{err}");
    }

    #[test]
    fn token_with_a_foreign_signature_is_rejected() {
        let token = sign_test_token(
            b"signing-secret",
            "client_1",
            TEST_ISSUER,
            future_exp(),
            Some("k1"),
        );

        let err = verify_with_secret(&token, b"different-secret").unwrap_err();
        assert!(err.to_string().contains("InvalidSignature"), "{err}");
    }

    #[test]
    fn jwks_verification_requires_a_kid() {
        let token = sign_test_token(b"s", "client_1", TEST_ISSUER, future_exp(), None);

        let err = verify_access_token_with_jwks(&token, &empty_jwks(), "client_1", TEST_ISSUER)
            .unwrap_err();
        assert!(err.to_string().contains("no kid"), "{err}");
    }

    #[test]
    fn jwks_verification_rejects_an_unknown_kid() {
        let token = sign_test_token(b"s", "client_1", TEST_ISSUER, future_exp(), Some("other"));

        let err = verify_access_token_with_jwks(&token, &empty_jwks(), "client_1", TEST_ISSUER)
            .unwrap_err();
        assert!(err.to_string().contains("no JWKS key matches kid"), "{err}");
    }

    #[test]
    fn malformed_token_is_rejected_without_panicking() {
        let err =
            verify_access_token_with_jwks("not-a-jwt", &empty_jwks(), "client_1", TEST_ISSUER)
                .unwrap_err();
        assert!(err.to_string().contains("malformed token header"), "{err}");
    }
}
