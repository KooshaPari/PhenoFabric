//! Auth protocol message handlers for the wire server.
//!
//! Handles the three auth message types defined in the wire schema:
//! - `auth_start`: begin WorkOS AuthKit login, returns the authorize URL
//! - `auth_complete`: exchange the authorization code for tokens
//! - `auth_email`: start passwordless Magic Auth login for an email address
//!
//! These handlers use the coordinator's configured WorkOS provider. They are
//! public routes (pre-auth bootstrap), so the middleware does not gate them.

use crate::coordinator::Coordinator;

/// Handle an `auth_start` message: generate the WorkOS authorize URL.
pub(crate) async fn handle_auth_start(
    coordinator: &Coordinator,
) -> Option<String> {
    let Some(provider) = coordinator.workos_provider() else {
        return Some(auth_config_error("workos_not_configured", "set auth.workos_client_id in daemon config"));
    };

    let request = provider.generate_auth_url();

    Some(format!(
        r#"{{"type":"auth_start_response","status":"ok","url":{},"state":{},"redirect_uri":{}}}"#,
        json_string(&request.url),
        json_string(&request.state),
        json_string(&coordinator.redirect_uri()),
    ))
}

/// Handle an `auth_complete` message: exchange the authorization code.
pub(crate) async fn handle_auth_complete(
    parsed: &serde_json::Value,
    coordinator: &Coordinator,
) -> Option<String> {
    let Some(code) = parsed.get("code").and_then(|v| v.as_str()) else {
        // Unreachable via validate_message (schema requires `code`), but keep
        // a defensive error for direct calls.
        return Some(auth_config_error("missing_code", "code field required"));
    };

    let Some(provider) = coordinator.workos_provider() else {
        return Some(auth_config_error("workos_not_configured", "set auth.workos_client_id in daemon config"));
    };

    match provider.exchange_code(code).await {
        Ok(tokens) => Some(format!(
            r#"{{"type":"auth_complete_response","status":"ok","user":{}}}"#,
            serde_json::to_string(&tokens.user).unwrap_or_else(|_| "{}".into())
        )),
        Err(e) => Some(format!(
            r#"{{"type":"auth_error","error":"exchange_failed","message":"{}"}}"#,
            e
        )),
    }
}

/// Handle an `auth_email` message: send a Magic Auth code to the email.
pub(crate) async fn handle_auth_email(
    parsed: &serde_json::Value,
    coordinator: &Coordinator,
) -> Option<String> {
    let Some(email) = parsed.get("email").and_then(|v| v.as_str()) else {
        // Unreachable via validate_message (schema requires `email`).
        return Some(auth_config_error("missing_email", "email field required"));
    };

    let Some(provider) = coordinator.workos_provider() else {
        return Some(auth_config_error("workos_not_configured", "set auth.workos_client_id in daemon config"));
    };

    match provider.send_magic_auth_code(email).await {
        Ok(_) => Some(format!(
            r#"{{"type":"auth_email_response","status":"ok","email":{}}}"#,
            json_string(email)
        )),
        Err(e) => Some(format!(
            r#"{{"type":"auth_error","error":"magic_auth_failed","message":"{}"}}"#,
            e
        )),
    }
}

/// Build a JSON error response for auth configuration problems.
fn auth_config_error(error: &str, message: &str) -> String {
    format!(
        r#"{{"type":"auth_error","error":"{error}","message":"{message}"}}"#
    )
}

/// Serialize a string as a JSON string literal.
fn json_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::protocol::process_message;
    use std::sync::Arc;

    fn make_coordinator() -> Arc<Coordinator> {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        Arc::new(Coordinator::new(config).unwrap())
    }

    fn make_coordinator_with_workos() -> Arc<Coordinator> {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            auth: crate::config::AuthConfig {
                workos_client_id: "test-client".into(),
                workos_client_secret: "test-secret".into(),
                workos_redirect_uri: "http://127.0.0.1:0/callback".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        Arc::new(Coordinator::new(config).unwrap())
    }

    #[tokio::test]
    async fn auth_start_without_workos_returns_error() {
        let coord = make_coordinator();
        let msg = r#"{"type":"auth_start"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("auth_error"));
        assert!(resp.contains("workos_not_configured"));
    }

    #[tokio::test]
    async fn auth_start_returns_authorize_url() {
        let coord = make_coordinator_with_workos();
        let msg = r#"{"type":"auth_start"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("auth_start_response"));
        assert!(resp.contains("status"));
        assert!(resp.contains("https://api.workos.com"));
        assert!(resp.contains("state"));
    }

    #[tokio::test]
    async fn auth_complete_without_workos_returns_error() {
        let coord = make_coordinator();
        let msg = r#"{"type":"auth_complete","code":"some-code"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("auth_error"));
        assert!(resp.contains("workos_not_configured"));
    }

    #[tokio::test]
    async fn auth_complete_with_bad_code_returns_exchange_error() {
        let coord = make_coordinator_with_workos();
        let msg = r#"{"type":"auth_complete","code":"invalid-code-123"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        // The exchange fails against the real WorkOS API (invalid client), so
        // the response must be an auth_error, never a panic or hang.
        assert!(resp.contains("auth_error") || resp.contains("exchange_failed"));
    }

    #[tokio::test]
    async fn auth_email_without_workos_returns_error() {
        let coord = make_coordinator();
        let msg = r#"{"type":"auth_email","email":"user@example.com"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("auth_error"));
        assert!(resp.contains("workos_not_configured"));
    }

    #[tokio::test]
    async fn auth_email_with_bad_config_returns_magic_auth_error() {
        let coord = make_coordinator_with_workos();
        let msg = r#"{"type":"auth_email","email":"user@example.com"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        // The send fails against the real WorkOS API (invalid client), so
        // the response must be an auth_error, never a panic or hang.
        assert!(resp.contains("auth_error") || resp.contains("magic_auth_failed"));
    }
}
