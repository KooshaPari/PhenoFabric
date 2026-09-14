//! JWT decoding and token hashing for auth middleware.

use serde::Deserialize;

use super::{AuthError, AuthenticatedUser};

/// JWT claims for decoding WorkOS-issued tokens.
#[derive(Debug, Deserialize)]
pub(super) struct JwtClaims {
    /// Subject (user ID).
    pub(super) sub: String,
    /// Email address.
    #[serde(default)]
    pub(super) email: Option<String>,
    /// Organization ID.
    #[serde(default)]
    pub(super) org_id: Option<String>,
    /// Expiration timestamp.
    #[serde(default)]
    pub(super) _exp: Option<u64>,
}

/// Decode a JWT token and extract user claims.
pub(super) fn decode_jwt(token: &str, secret: &str) -> Result<AuthenticatedUser, AuthError> {
    let token_data = jsonwebtoken::decode::<JwtClaims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &jsonwebtoken::Validation::default(),
    )
    .map_err(|e| AuthError::JwtDecode(e.to_string()))?;

    let claims = token_data.claims;

    Ok(AuthenticatedUser {
        user_id: claims.sub,
        email: claims.email.unwrap_or_default(),
        org_id: claims.org_id,
    })
}

/// Simple hash function for token caching.
pub(super) fn token_hash(token: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    hasher.finish()
}
