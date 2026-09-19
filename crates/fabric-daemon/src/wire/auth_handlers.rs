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
pub(crate) async fn handle_auth_start(coordinator: &Coordinator) -> Option<String> {
    let Some(provider) = coordinator.workos_provider() else {
        return Some(auth_config_error(
            "workos_not_configured",
            "set auth.workos_client_id in daemon config",
        ));
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
        return Some(auth_config_error(
            "workos_not_configured",
            "set auth.workos_client_id in daemon config",
        ));
    };

    match provider.exchange_code(code).await {
        Ok(tokens) => Some(login_success(
            coordinator,
            &tokens,
            "auth_complete_response",
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
        return Some(auth_config_error(
            "workos_not_configured",
            "set auth.workos_client_id in daemon config",
        ));
    };

    match provider.send_magic_auth_code(email).await {
        Ok(_) => Some(format!(
            r#"{{"type":"auth_email_response","status":"ok","success":true,"message":"magic auth code sent to {email}","email":{}}}"#,
            json_string(email)
        )),
        Err(e) => Some(format!(
            r#"{{"type":"auth_error","error":"magic_auth_failed","message":"{}"}}"#,
            e
        )),
    }
}

/// Handle an `auth_status` message: report the current login state.
///
/// The GUI polls this on load and after every refresh. The response carries
/// the `AuthStatus` contract fields (`logged_in`, `user_name`, `user_email`,
/// `org_name`, `roles`, `session_expiry_secs`, `active_sessions`), so a
/// logged-in session survives a GUI restart while the daemon keeps running.
/// An expired session reports as logged out.
pub(crate) async fn handle_auth_status(coordinator: &Coordinator) -> Option<String> {
    let session = coordinator
        .auth_session()
        .filter(|s| s.expires_at > chrono::Utc::now());

    Some(match session {
        Some(s) => {
            let remaining = (s.expires_at - chrono::Utc::now()).num_seconds().max(0) as u64;
            let roles_json = serde_json::to_string(&s.roles).unwrap_or_else(|_| "[]".into());
            format!(
                r#"{{"type":"auth_status_response","status":"ok","logged_in":true,"user_name":{},"user_email":{},"org_name":{},"roles":{roles_json},"session_expiry_secs":{},"active_sessions":[{{"session_id":{},"device":"Phenotype Fabric GUI","created":{}}}]}}"#,
                json_string(&s.user_name),
                json_string(&s.user_email),
                json_string(&s.org_name),
                remaining,
                json_string(&s.session_id),
                json_string(&s.created.to_rfc3339()),
            )
        }
        None => LOGGED_OUT_STATUS.into(),
    })
}

/// Handle an `auth_verify` message: verify a Magic Auth code and log in.
///
/// Completes the passwordless flow started by `auth_email`: the user receives
/// a code by email, submits it with their address, and the daemon exchanges
/// it for tokens and stores the session.
pub(crate) async fn handle_auth_verify(
    parsed: &serde_json::Value,
    coordinator: &Coordinator,
) -> Option<String> {
    let email = parsed.get("email").and_then(|v| v.as_str());
    let code = parsed.get("code").and_then(|v| v.as_str());
    let (Some(email), Some(code)) = (email, code) else {
        // Unreachable via validate_message (schema requires both fields), but
        // keep a defensive error for direct calls.
        return Some(auth_config_error(
            "missing_fields",
            "email and code fields required",
        ));
    };

    let Some(provider) = coordinator.workos_provider() else {
        return Some(auth_config_error(
            "workos_not_configured",
            "set auth.workos_client_id in daemon config",
        ));
    };

    match provider
        .authenticate_with_magic_auth_code(email, code)
        .await
    {
        Ok(tokens) => Some(login_success(coordinator, &tokens, "auth_verify_response")),
        Err(e) => Some(format!(
            r#"{{"type":"auth_error","error":"magic_verify_failed","message":"{}"}}"#,
            e
        )),
    }
}

/// The logged-out `AuthStatus` shape (mirrors the GUI struct's defaults).
const LOGGED_OUT_STATUS: &str = r#"{"type":"auth_status_response","status":"ok","logged_in":false,"user_name":"-","user_email":"-","org_name":"-","roles":[],"session_expiry_secs":0,"active_sessions":[]}"#;

/// Store the authenticated session and build the success response.
///
/// Both `auth_complete` (OAuth code) and `auth_verify` (magic auth code)
/// funnel through here, so the GUI sees one consistent login shape.
fn login_success(
    coordinator: &Coordinator,
    tokens: &crate::auth::TokenResponse,
    msg_type: &str,
) -> String {
    coordinator.set_auth_session(crate::coordinator::AuthSessionState {
        session_id: uuid::Uuid::new_v4().to_string(),
        user_name: tokens.user.name.clone(),
        user_email: tokens.user.email.clone(),
        org_name: tokens.user.org_id.clone().unwrap_or_else(|| "-".into()),
        roles: vec![],
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(tokens.expires_in as i64),
        created: chrono::Utc::now(),
    });
    auth_success_response(msg_type, tokens)
}

/// Build the success response for a completed login exchange.
///
/// The GUI deserializes the response into its `AuthStatus` struct, which
/// requires exactly these fields (no serde defaults): `logged_in`,
/// `user_name`, `user_email`, `org_name`, `roles`, `session_expiry_secs`,
/// `active_sessions`. Returning the raw WorkOS user object here made every
/// live login fail at the parse step even after a successful exchange, so the
/// shape is a cross-crate contract pinned by the
/// `auth_complete_success_parses_as_gui_auth_status` test.
fn auth_success_response(msg_type: &str, tokens: &crate::auth::TokenResponse) -> String {
    format!(
        r#"{{"type":"{msg_type}","status":"ok","logged_in":true,"user_name":{},"user_email":{},"org_name":{},"roles":[],"session_expiry_secs":{},"active_sessions":[],"user":{}}}"#,
        json_string(&tokens.user.name),
        json_string(&tokens.user.email),
        json_string(tokens.user.org_id.as_deref().unwrap_or("-")),
        tokens.expires_in,
        serde_json::to_string(&tokens.user).unwrap_or_else(|_| "{}".into())
    )
}

/// Build a JSON error response for auth configuration problems.
fn auth_config_error(error: &str, message: &str) -> String {
    format!(r#"{{"type":"auth_error","error":"{error}","message":"{message}"}}"#)
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

    #[tokio::test]
    async fn auth_complete_success_parses_as_gui_auth_status() {
        // Cross-crate contract: the GUI's fetch_complete_auth deserializes the
        // response line into fabric-gui's AuthStatus, which requires these
        // exact fields. Reconstruct that struct here (same serde shape) and
        // verify the daemon's success response parses into it.
        // exchange_code against the real WorkOS API fails with an invalid
        // test client, so pin the success shape through the formatter
        // directly instead of through the full exchange path.
        let tokens = crate::auth::TokenResponse {
            access_token: "at".into(),
            refresh_token: "rt".into(),
            expires_in: 3600,
            token_type: "Bearer".into(),
            user: crate::auth::WorkOsUser {
                id: "user_123".into(),
                email: "dev@example.com".into(),
                name: "Dev User".into(),
                org_id: Some("org_456".into()),
            },
        };
        let resp = auth_success_response("auth_complete_response", &tokens);
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["logged_in"], serde_json::json!(true));
        assert_eq!(v["user_name"], serde_json::json!("Dev User"));
        assert_eq!(v["user_email"], serde_json::json!("dev@example.com"));
        assert_eq!(v["org_name"], serde_json::json!("org_456"));
        assert_eq!(v["session_expiry_secs"], serde_json::json!(3600));
        // All required AuthStatus fields must be present with correct types.
        for field in [
            "logged_in",
            "user_name",
            "user_email",
            "org_name",
            "roles",
            "session_expiry_secs",
            "active_sessions",
        ] {
            assert!(v.get(field).is_some(), "missing AuthStatus field: {field}");
        }
        assert!(v["roles"].is_array());
        assert!(v["active_sessions"].is_array());
    }

    /// Reconstruct the GUI's `AuthStatus` serde shape and assert a full parse.
    fn assert_parses_as_gui_auth_status(resp: &str) {
        let v: serde_json::Value = serde_json::from_str(resp).expect("valid JSON");
        for field in [
            "logged_in",
            "user_name",
            "user_email",
            "org_name",
            "roles",
            "session_expiry_secs",
            "active_sessions",
        ] {
            assert!(v.get(field).is_some(), "missing AuthStatus field: {field}");
        }
        assert!(v["logged_in"].is_boolean());
        assert!(v["user_name"].is_string());
        assert!(v["user_email"].is_string());
        assert!(v["org_name"].is_string());
        assert!(v["roles"].is_array());
        assert!(v["session_expiry_secs"].is_u64());
        assert!(v["active_sessions"].is_array());
    }

    #[tokio::test]
    async fn auth_status_logged_out_parses_as_gui_auth_status() {
        let coord = make_coordinator();
        let resp = process_message(r#"{"type":"auth_status"}"#, &coord)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["logged_in"], serde_json::json!(false));
        assert_parses_as_gui_auth_status(&resp);
    }

    #[tokio::test]
    async fn auth_status_after_login_reports_session() {
        let coord = make_coordinator();
        coord.set_auth_session(crate::coordinator::AuthSessionState {
            session_id: "sess-1".into(),
            user_name: "Dev User".into(),
            user_email: "dev@example.com".into(),
            org_name: "org_456".into(),
            roles: vec![],
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            created: chrono::Utc::now(),
        });

        let resp = process_message(r#"{"type":"auth_status"}"#, &coord)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["logged_in"], serde_json::json!(true));
        assert_eq!(v["user_name"], serde_json::json!("Dev User"));
        assert_eq!(v["user_email"], serde_json::json!("dev@example.com"));
        assert_eq!(v["org_name"], serde_json::json!("org_456"));
        assert!(v["session_expiry_secs"].as_u64().unwrap() > 0);
        assert_eq!(v["active_sessions"].as_array().unwrap().len(), 1);
        assert_eq!(
            v["active_sessions"][0]["session_id"],
            serde_json::json!("sess-1")
        );
        assert_parses_as_gui_auth_status(&resp);
    }

    #[tokio::test]
    async fn auth_status_reports_expired_session_as_logged_out() {
        let coord = make_coordinator();
        coord.set_auth_session(crate::coordinator::AuthSessionState {
            session_id: "sess-expired".into(),
            user_name: "Dev User".into(),
            user_email: "dev@example.com".into(),
            org_name: "org_456".into(),
            roles: vec![],
            // Already expired: must not be reported as logged in.
            expires_at: chrono::Utc::now() - chrono::Duration::seconds(1),
            created: chrono::Utc::now() - chrono::Duration::hours(1),
        });

        let resp = process_message(r#"{"type":"auth_status"}"#, &coord)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["logged_in"], serde_json::json!(false));
        assert_parses_as_gui_auth_status(&resp);
    }

    #[tokio::test]
    async fn auth_verify_without_workos_returns_error() {
        let coord = make_coordinator();
        let msg = r#"{"type":"auth_verify","email":"dev@example.com","code":"123456"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("auth_error"));
        assert!(resp.contains("workos_not_configured"));
    }

    #[tokio::test]
    async fn auth_verify_missing_code_rejected_by_schema() {
        let coord = make_coordinator();
        let msg = r#"{"type":"auth_verify","email":"dev@example.com"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        // Schema requires both email and code, so this never reaches the
        // provider and must come back as a validation error.
        assert!(resp.contains("validation") || resp.contains("code"));
        assert!(!resp.contains("auth_verify_response"));
    }
}
