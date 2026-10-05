//! Tests for the sibling `secrets` module. See `super` for the code.
use super::*;

#[test]
fn default_config_is_empty() {
    let config = InfisicalConfig::default();
    assert!(config.client_id.is_empty());
    assert!(config.client_secret.is_empty());
    assert!(config.project_id.is_empty());
    assert_eq!(config.base_url, DEFAULT_INFISICAL_BASE_URL);
}

#[test]
fn default_config_uses_cli_documented_us_cloud_api_host() {
    let config = InfisicalConfig::default();
    assert_eq!(config.base_url, "https://app.infisical.com");
}

#[test]
fn secret_value_serialization_roundtrip() {
    let sv = SecretValue {
        key: "DB_PASSWORD".into(),
        value: "s3cret".into(),
        environment: "prod".into(),
        path: Some("/database".into()),
    };

    let json = serde_json::to_string(&sv).unwrap();
    let deserialized: SecretValue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.key, "DB_PASSWORD");
    assert_eq!(deserialized.value, "s3cret");
    assert_eq!(deserialized.environment, "prod");
}

#[test]
fn secret_value_optional_path() {
    let json = r#"{"key":"K","value":"V","environment":"dev"}"#;
    let sv: SecretValue = serde_json::from_str(json).unwrap();
    assert!(sv.path.is_none());
}

/// Official docs (observed 2026-09-27,
/// https://infisical.com/docs/api-reference/endpoints/universal-auth/login)
/// return `accessToken`/`expiresIn`/`tokenType`; the snake_case fixture
/// shape no longer describes the live contract.
#[test]
fn token_response_matches_documented_camelcase_shape() {
    let docs_json =
        r#"{"accessToken":"tok","expiresIn":300,"accessTokenMaxTTL":600,"tokenType":"Bearer"}"#;
    let parsed: TokenResponse =
        serde_json::from_str(docs_json).expect("documented shape must parse");
    assert_eq!(parsed.access_token, "tok");
    assert_eq!(parsed.expires_in, 300);
    assert_eq!(parsed._token_type, "Bearer");
}

// -------------------------------------------------------- v1 token eviction
//
// The legacy v1 read/write paths all build their `Authorization` header from
// `ensure_token`. A 401/403 therefore means the cached token was refused, and
// leaving it cached replays the refused token for the rest of its TTL. The
// tests below drive a local in-process fake of the v1 API and count logins:
// a re-authentication is the only way to tell a fresh attempt from a replay
// (both produce two reads). No real Infisical, no real credentials.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Marker placed in the rejected response body; it must never surface in an
/// error's display or debug rendering.
const V1_BODY_MARKER: &str = "BODY-MARKER-NEVER-LEAK-v1-9c2f";

/// Fake v1 API: answers the universal-auth login, rejects the first non-login
/// request with `op_status`, then serves a valid secret read. Records login and
/// non-login counts. Login is identified by its path, so a write POST is still
/// counted as an operation.
fn flaky_v1_server(op_status: &'static str) -> (String, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let ops: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let logins: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let recorded_ops = Arc::clone(&ops);
    let recorded_logins = Arc::clone(&logins);

    std::thread::spawn(move || {
        for _ in 0..4 {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut data = Vec::new();
            let mut chunk = [0u8; 1024];
            let head_end = loop {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break 0;
                }
                data.extend_from_slice(&chunk[..n]);
                if let Some(p) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    break p + 4;
                }
            };
            if head_end == 0 {
                continue;
            }
            let head = String::from_utf8_lossy(&data[..head_end]).to_string();
            // Drain the request body so the client never sees a reset.
            let content_length = head
                .to_ascii_lowercase()
                .lines()
                .find(|l| l.starts_with("content-length:"))
                .and_then(|l| l.split(':').nth(1))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            let mut body_remaining =
                content_length.saturating_sub(data.len().saturating_sub(head_end));
            while body_remaining > 0 {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break;
                }
                body_remaining = body_remaining.saturating_sub(n);
            }

            let request_line = head.lines().next().unwrap_or_default().to_string();
            let is_login = request_line.contains("/auth/universal-auth/login");
            let (status, body): (String, String) = if is_login {
                recorded_logins.fetch_add(1, Ordering::SeqCst);
                (
                    "200 OK".to_string(),
                    r#"{"accessToken":"fake-token","expiresIn":300,"accessTokenMaxTTL":600,"tokenType":"Bearer"}"#
                        .to_string(),
                )
            } else {
                let n = recorded_ops.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    (
                        op_status.to_string(),
                        format!(r#"{{"marker":"{V1_BODY_MARKER}"}}"#),
                    )
                } else {
                    (
                        "200 OK".to_string(),
                        r#"{"secret":{"secretKey":"DB_PASSWORD","secretValue":"from-fake-server"}}"#
                            .to_string(),
                    )
                }
            };

            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    (format!("http://{addr}"), ops, logins)
}

fn v1_client_for(base_url: String) -> InfisicalClient {
    InfisicalClient::new(InfisicalConfig {
        client_id: "sa-id".into(),
        client_secret: "sa-secret".into(),
        project_id: "test-project".into(),
        base_url,
    })
}

fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(future)
}

/// A 401 on the legacy v1 read must evict the cached token, so the next read
/// re-authenticates instead of replaying the token the server just refused.
#[test]
fn v1_read_401_evicts_token_and_next_read_reauthenticates() {
    let (base_url, ops, logins) = flaky_v1_server("401 Unauthorized");
    let mut client = v1_client_for(base_url);

    let err = block_on(client.get_secret("/db/password", "dev")).expect_err("401 must be an error");
    let rendered = format!("{err} | {err:?}");
    assert!(
        matches!(err, SecretsError::Auth(_)),
        "401 must map to SecretsError::Auth, got: {rendered}"
    );
    assert!(
        !rendered.contains(V1_BODY_MARKER),
        "response body leaked into error: {rendered}"
    );

    let second =
        block_on(client.get_secret("/db/password", "dev")).expect("second read must recover");
    assert_eq!(second.value, "from-fake-server");
    assert_eq!(
        ops.load(Ordering::SeqCst),
        2,
        "the second read must be a fresh attempt"
    );
    // Two reads alone cannot distinguish a re-auth from a cached-token replay.
    // The login count is what proves the refused token was not replayed.
    assert_eq!(
        logins.load(Ordering::SeqCst),
        2,
        "the second read must re-authenticate; a single login means the \
         refused token was replayed"
    );
}

/// 403 is the same failure mode (token valid, secret not permitted) and must
/// evict too, so a later permission change is not masked by a poisoned cache.
#[test]
fn v1_read_403_evicts_token_and_next_read_reauthenticates() {
    let (base_url, ops, logins) = flaky_v1_server("403 Forbidden");
    let mut client = v1_client_for(base_url);

    let err = block_on(client.get_secret("/db/password", "dev")).expect_err("403 must be an error");
    assert!(
        matches!(err, SecretsError::Auth(_)),
        "403 must map to SecretsError::Auth, got: {err}"
    );

    let second =
        block_on(client.get_secret("/db/password", "dev")).expect("second read must recover");
    assert_eq!(second.value, "from-fake-server");
    assert_eq!(ops.load(Ordering::SeqCst), 2);
    assert_eq!(logins.load(Ordering::SeqCst), 2);
}

/// The eviction is scoped to rejected credentials: a non-auth failure (500)
/// must NOT discard a token that is still valid, so the next call reuses it.
#[test]
fn v1_read_500_keeps_valid_token() {
    let (base_url, ops, logins) = flaky_v1_server("500 Internal Server Error");
    let mut client = v1_client_for(base_url);

    let _ = block_on(client.get_secret("/db/password", "dev")).expect_err("500 must be an error");
    let second =
        block_on(client.get_secret("/db/password", "dev")).expect("second read must recover");
    assert_eq!(second.value, "from-fake-server");
    assert_eq!(ops.load(Ordering::SeqCst), 2);
    assert_eq!(
        logins.load(Ordering::SeqCst),
        1,
        "a 500 is not a credential rejection and must not evict the token"
    );
}

/// The write path carries the same cached token, so a 401 there poisons the
/// cache identically; it must evict too.
#[test]
fn v1_write_401_evicts_token_and_next_call_reauthenticates() {
    let (base_url, ops, logins) = flaky_v1_server("401 Unauthorized");
    let mut client = v1_client_for(base_url);

    let err = block_on(client.set_secret("DB_PASSWORD", "s3cret", "dev"))
        .expect_err("401 must be an error");
    assert!(
        matches!(err, SecretsError::Auth(_)),
        "write 401 must map to SecretsError::Auth, got: {err}"
    );

    let next = block_on(client.get_secret("/db/password", "dev")).expect("next call must recover");
    assert_eq!(next.value, "from-fake-server");
    assert_eq!(ops.load(Ordering::SeqCst), 2);
    assert_eq!(
        logins.load(Ordering::SeqCst),
        2,
        "a 401 on a write must evict so the next call re-authenticates"
    );
}
