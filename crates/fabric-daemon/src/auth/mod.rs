//! Authentication module for fabric-daemon.
//!
//! Provides WorkOS OAuth 2.0 integration and Infisical secrets management
//! for securing wire transport and daemon operations.

pub mod middleware;
pub mod oauth;
pub mod secrets;

pub use middleware::{AuthMiddleware, AuthenticatedUser, AuthError, AuthMiddlewareConfig};
pub use oauth::{WorkOsProvider, WorkOsConfig, OAuthConfig, AuthorizationRequest, TokenResponse, WorkOsUser};
pub use secrets::{InfisicalClient, InfisicalConfig, SecretValue, SecretsError};
