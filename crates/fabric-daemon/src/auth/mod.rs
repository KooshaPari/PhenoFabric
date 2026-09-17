//! Authentication module for fabric-daemon.
//!
//! Provides WorkOS OAuth 2.0 integration and Infisical secrets management
//! for securing wire transport and daemon operations.

pub mod middleware;
pub mod oauth;
pub mod secrets;

#[allow(unused_imports)]
pub use middleware::{AuthError, AuthMiddleware, AuthMiddlewareConfig, AuthenticatedUser};
#[allow(unused_imports)]
pub use oauth::{
    AuthorizationRequest, OAuthConfig, TokenResponse, WorkOsConfig, WorkOsProvider, WorkOsUser,
};
#[allow(unused_imports)]
pub use secrets::{InfisicalClient, InfisicalConfig, SecretValue, SecretsError};
