//! Auth error response formatting for wire protocol messages.

use super::AuthError;

/// Build an auth error response for the wire protocol.
pub fn auth_error_response(error: &AuthError) -> String {
    let (error_code, message) = match error {
        AuthError::MissingToken => ("MISSING_TOKEN", error.to_string()),
        AuthError::InvalidTokenFormat => ("INVALID_TOKEN_FORMAT", error.to_string()),
        AuthError::TokenValidation(msg) => ("TOKEN_VALIDATION_FAILED", msg.clone()),
        AuthError::TokenExpired => ("TOKEN_EXPIRED", error.to_string()),
        AuthError::UserNotFound => ("USER_NOT_FOUND", error.to_string()),
        AuthError::JwtDecode(msg) => ("JWT_DECODE_ERROR", msg.clone()),
        AuthError::Introspection(msg) => ("INTROSPECTION_ERROR", msg.clone()),
        AuthError::Disabled => ("AUTH_DISABLED", error.to_string()),
    };

    serde_json::json!({
        "type": "auth_error",
        "error": error_code,
        "message": message,
    })
    .to_string()
}
