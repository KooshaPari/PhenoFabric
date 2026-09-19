//! Authentication middleware for fabric-daemon wire transport.
//!
//! Validates Bearer tokens from wire protocol messages and request contexts.
//! Supports both local JWT decoding and WorkOS JWKS verification.
//!
//! Public routes (`health_check`, `status_check`) skip authentication.

#![allow(dead_code)]

mod jwt;
pub mod routes;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use self::jwt::{decode_jwt, token_hash};
use super::oauth::{WorkOsConfig, WorkOsProvider};

/// Errors that can occur during authentication.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("no authorization token provided")]
    MissingToken,

    #[error("invalid token format")]
    InvalidTokenFormat,

    #[error("token validation failed: {0}")]
    TokenValidation(String),

    #[error("token has expired")]
    TokenExpired,

    #[error("user not found for token")]
    UserNotFound,

    #[error("JWT decode error: {0}")]
    JwtDecode(String),

    #[error("token verification failed: {0}")]
    TokenVerification(String),

    #[error("authentication disabled")]
    Disabled,
}

/// An authenticated user extracted from a valid token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    /// The user's unique identifier.
    pub user_id: String,
    /// The user's email address.
    pub email: String,
    /// The user's organization ID, if applicable.
    pub org_id: Option<String>,
}

/// Configuration for the auth middleware.
#[derive(Debug, Clone)]
pub struct AuthMiddlewareConfig {
    /// Whether authentication is enabled.
    pub enabled: bool,
    /// Message types that are exempt from authentication.
    pub public_routes: HashSet<String>,
    /// WorkOS configuration for token verification.
    pub workos_config: Option<WorkOsConfig>,
    /// JWT secret for local token decoding (alternative to WorkOS verification).
    pub jwt_secret: Option<String>,
}

impl Default for AuthMiddlewareConfig {
    fn default() -> Self {
        let mut public_routes = HashSet::new();
        public_routes.insert("health_check".to_string());
        public_routes.insert("status_check".to_string());

        Self {
            enabled: false,
            public_routes,
            workos_config: None,
            jwt_secret: None,
        }
    }
}

/// Authentication middleware for wire protocol messages.
///
/// Extracts and validates Bearer tokens from incoming messages.
/// Caches user lookups for performance.
#[derive(Debug)]
pub struct AuthMiddleware {
    config: AuthMiddlewareConfig,
    provider: Option<WorkOsProvider>,
    _http: Client,
    /// Cache of token -> user for performance.
    user_cache: Arc<RwLock<Vec<CachedUser>>>,
}

/// Cached user entry with expiry.
#[derive(Debug, Clone)]
struct CachedUser {
    token_hash: u64,
    user: AuthenticatedUser,
    expires_at: std::time::Instant,
}

impl AuthMiddleware {
    /// Create a new auth middleware.
    pub fn new(config: AuthMiddlewareConfig) -> Self {
        let provider = config
            .workos_config
            .as_ref()
            .map(|c| WorkOsProvider::new(c.clone()));

        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to create HTTP client");

        Self {
            config,
            provider,
            _http: http,
            user_cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new auth middleware with a custom HTTP client (for testing).
    pub fn with_client(config: AuthMiddlewareConfig, http: Client) -> Self {
        let provider = config
            .workos_config
            .as_ref()
            .map(|c| WorkOsProvider::with_client(c.clone(), http.clone()));

        Self {
            config,
            provider,
            _http: http,
            user_cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if a message type requires authentication.
    pub fn is_public_route(&self, message_type: &str) -> bool {
        self.config.public_routes.contains(message_type)
    }

    /// Validate a message and extract the authenticated user.
    ///
    /// Returns `Ok(None)` for public routes that don't require auth.
    /// Returns `Ok(Some(user))` for authenticated requests.
    /// Returns `Err` if auth is required but fails.
    pub async fn validate_message(
        &self,
        message: &serde_json::Value,
    ) -> Result<Option<AuthenticatedUser>, AuthError> {
        // If auth is disabled, pass everything through.
        if !self.config.enabled {
            return Ok(None);
        }

        // Check if this is a public route.
        let msg_type = message.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if self.is_public_route(msg_type) {
            return Ok(None);
        }

        // Extract token from message.
        let token = self.extract_token(message)?;
        self.validate_token(&token).await
    }

    /// Extract a Bearer token from a wire message.
    ///
    /// Checks these fields in order:
    /// 1. `message.auth.token` (structured auth object)
    /// 2. `message.token` (top-level token field)
    /// 3. `message.headers.authorization` (HTTP-style header)
    fn extract_token(&self, message: &serde_json::Value) -> Result<String, AuthError> {
        // Check nested auth.token
        if let Some(auth) = message.get("auth") {
            if let Some(token) = auth.get("token").and_then(|v| v.as_str()) {
                return parse_bearer_token(token);
            }
        }

        // Check top-level token
        if let Some(token) = message.get("token").and_then(|v| v.as_str()) {
            return parse_bearer_token(token);
        }

        // Check headers.authorization
        if let Some(headers) = message.get("headers") {
            if let Some(auth) = headers.get("authorization").and_then(|v| v.as_str()) {
                return parse_bearer_token(auth);
            }
        }

        Err(AuthError::MissingToken)
    }

    /// Validate a token and return the authenticated user.
    pub async fn validate_token(
        &self,
        token: &str,
    ) -> Result<Option<AuthenticatedUser>, AuthError> {
        // Check cache first.
        {
            let cache = self.user_cache.read().await;
            if let Some(entry) = cache.iter().find(|e| {
                e.token_hash == token_hash(token) && std::time::Instant::now() < e.expires_at
            }) {
                return Ok(Some(entry.user.clone()));
            }
        }

        // Try local JWT decode first if we have a secret.
        if let Some(ref jwt_secret) = self.config.jwt_secret {
            if let Ok(user) = decode_jwt(token, jwt_secret) {
                self.cache_user(token, &user, 300).await;
                return Ok(Some(user));
            }
        }

        // Fall back to WorkOS token verification. AuthKit access tokens are
        // RS256 JWTs verified against the client's JWKS; WorkOS exposes no
        // introspection endpoint to call instead.
        if let Some(ref provider) = self.provider {
            return match provider.verify_access_token(token).await {
                Ok(claims) => {
                    let user = AuthenticatedUser {
                        user_id: claims.sub,
                        // AuthKit access tokens carry no email claim.
                        email: String::new(),
                        org_id: claims.org_id,
                    };
                    self.cache_user(token, &user, 300).await;
                    Ok(Some(user))
                }
                Err(e) => Err(AuthError::TokenVerification(e.to_string())),
            };
        }

        Err(AuthError::TokenValidation(
            "no validation method configured".to_string(),
        ))
    }

    /// Cache a validated user for the given token.
    async fn cache_user(&self, token: &str, user: &AuthenticatedUser, ttl_secs: u64) {
        let entry = CachedUser {
            token_hash: token_hash(token),
            user: user.clone(),
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(ttl_secs),
        };

        let mut cache = self.user_cache.write().await;

        // Evict expired entries.
        cache.retain(|e| std::time::Instant::now() < e.expires_at);

        // Remove any existing entry for this token.
        cache.retain(|e| e.token_hash != entry.token_hash);

        cache.push(entry);
    }

    /// Clear the user cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.user_cache.write().await;
        cache.clear();
    }
}

/// Extract the current authenticated user from a wire message.
///
/// This is a convenience function that can be called from handlers
/// to get the authenticated user if present in the request extensions.
pub fn get_current_user(message: &serde_json::Value) -> Option<AuthenticatedUser> {
    message
        .get("_auth_user")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// Set the authenticated user in a wire message for downstream handlers.
pub fn set_current_user(
    message: &mut serde_json::Value,
    user: &AuthenticatedUser,
) -> Result<(), std::io::Error> {
    let user_value = serde_json::to_value(user)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    message
        .as_object_mut()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot set auth user on non-object message",
            )
        })?
        .insert("_auth_user".to_string(), user_value);
    Ok(())
}

/// Parse a Bearer token from an authorization header value.
fn parse_bearer_token(header: &str) -> Result<String, AuthError> {
    let header = header.trim();
    if header.is_empty() {
        return Err(AuthError::MissingToken);
    }
    if let Some(token) = header.strip_prefix("Bearer ") {
        let token = token.trim();
        if token.is_empty() {
            return Err(AuthError::InvalidTokenFormat);
        }
        Ok(token.to_string())
    } else if header.eq_ignore_ascii_case("Bearer") {
        // "Bearer" without value.
        Err(AuthError::InvalidTokenFormat)
    } else {
        // Allow raw tokens (not prefixed with "Bearer ").
        Ok(header.to_string())
    }
}

/// Attach authentication to a message by adding auth fields.
///
/// This is used by clients to include authentication in wire messages.
pub fn attach_auth(message: &mut serde_json::Value, token: &str) -> Result<(), std::io::Error> {
    let obj = message.as_object_mut().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "cannot attach auth to non-object message",
        )
    })?;

    obj.insert("auth".to_string(), serde_json::json!({ "token": token }));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bearer_token_valid() {
        let token = parse_bearer_token("Bearer abc123").unwrap();
        assert_eq!(token, "abc123");
    }

    #[test]
    fn parse_bearer_token_raw() {
        let token = parse_bearer_token("xyz789").unwrap();
        assert_eq!(token, "xyz789");
    }

    #[test]
    fn parse_bearer_token_empty() {
        let err = parse_bearer_token("Bearer ").unwrap_err();
        assert!(matches!(err, AuthError::InvalidTokenFormat));
    }

    #[test]
    fn parse_bearer_token_missing() {
        let err = parse_bearer_token("").unwrap_err();
        assert!(matches!(err, AuthError::MissingToken));
    }

    #[test]
    fn is_public_route() {
        let config = AuthMiddlewareConfig::default();
        let middleware = AuthMiddleware::new(config);

        assert!(middleware.is_public_route("health_check"));
        assert!(middleware.is_public_route("status_check"));
        assert!(!middleware.is_public_route("compile_request"));
        assert!(!middleware.is_public_route("webrtc_offer"));
    }

    #[test]
    fn extract_token_from_auth_field() {
        let msg = serde_json::json!({
            "type": "compile_request",
            "auth": { "token": "Bearer mytoken123" }
        });

        let middleware = AuthMiddleware::new(AuthMiddlewareConfig {
            enabled: true,
            ..Default::default()
        });

        let token = middleware.extract_token(&msg).unwrap();
        assert_eq!(token, "mytoken123");
    }

    #[test]
    fn extract_token_from_top_level() {
        let msg = serde_json::json!({
            "type": "compile_request",
            "token": "raw-token-456"
        });

        let middleware = AuthMiddleware::new(AuthMiddlewareConfig {
            enabled: true,
            ..Default::default()
        });

        let token = middleware.extract_token(&msg).unwrap();
        assert_eq!(token, "raw-token-456");
    }

    #[test]
    fn extract_token_from_headers() {
        let msg = serde_json::json!({
            "type": "compile_request",
            "headers": { "authorization": "Bearer header-token" }
        });

        let middleware = AuthMiddleware::new(AuthMiddlewareConfig {
            enabled: true,
            ..Default::default()
        });

        let token = middleware.extract_token(&msg).unwrap();
        assert_eq!(token, "header-token");
    }

    #[test]
    fn extract_token_missing_returns_error() {
        let msg = serde_json::json!({
            "type": "compile_request"
        });

        let middleware = AuthMiddleware::new(AuthMiddlewareConfig {
            enabled: true,
            ..Default::default()
        });

        let err = middleware.extract_token(&msg).unwrap_err();
        assert!(matches!(err, AuthError::MissingToken));
    }

    #[tokio::test]
    async fn auth_disabled_passes_all() {
        let config = AuthMiddlewareConfig::default();
        let middleware = AuthMiddleware::new(config);

        let result = middleware
            .validate_message(&serde_json::json!({"type": "compile_request"}))
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn public_route_passes_without_token() {
        let config = AuthMiddlewareConfig {
            enabled: true,
            ..Default::default()
        };
        let middleware = AuthMiddleware::new(config);

        let result = middleware
            .validate_message(&serde_json::json!({"type": "health_check"}))
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn protected_route_requires_token() {
        let config = AuthMiddlewareConfig {
            enabled: true,
            ..Default::default()
        };
        let middleware = AuthMiddleware::new(config);

        let result = middleware
            .validate_message(&serde_json::json!({"type": "compile_request"}))
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::MissingToken));
    }

    #[test]
    fn auth_error_response_format() {
        let resp = routes::auth_error_response(&AuthError::MissingToken);
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["type"], "auth_error");
        assert_eq!(parsed["error"], "MISSING_TOKEN");
    }

    #[test]
    fn attach_and_get_user() {
        let mut msg = serde_json::json!({"type": "compile_request"});
        let user = AuthenticatedUser {
            user_id: "u1".into(),
            email: "a@b.com".into(),
            org_id: Some("o1".into()),
        };

        set_current_user(&mut msg, &user).unwrap();
        let retrieved = get_current_user(&msg).unwrap();
        assert_eq!(retrieved.user_id, "u1");
        assert_eq!(retrieved.email, "a@b.com");
        assert_eq!(retrieved.org_id, Some("o1".into()));
    }

    #[test]
    fn token_hash_deterministic() {
        let h1 = token_hash("abc");
        let h2 = token_hash("abc");
        assert_eq!(h1, h2);

        let h3 = token_hash("def");
        assert_ne!(h1, h3);
    }
}
