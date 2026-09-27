//! Local-only verification of the production Infisical fetch adapter.
//!
//! Exercises [`super::fetch_via_infisical`] against an in-process fake of
//! the official API surface: universal-auth login plus the v4 named-secret
//! read (docs observed 2026-09-24). Never contacts real Infisical and
//! never uses real credentials.

use super::*;

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// Small deterministic fake of the Infisical HTTP API (local only).
/// Serves the universal-auth login POST and the v4 secret GET, recording
/// request lines. Responds `Connection: close` per request.
///
/// Robust against aborted/empty pool connections (skipped, never fatal)
/// so a stray connect cannot take the listener down mid-test.
fn fake_infisical_server() -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requests: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&requests);

    std::thread::spawn(move || {
        for _ in 0..32 {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
            let mut data = Vec::new();
            let mut chunk = [0u8; 1024];
            let head_end = loop {
                let Ok(n) = stream.read(&mut chunk) else {
                    break 0;
                };
                if n == 0 {
                    break 0;
                }
                data.extend_from_slice(&chunk[..n]);
                if let Some(p) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    break p + 4;
                }
                if data.len() > 64 * 1024 {
                    break 0;
                }
            };
            if head_end == 0 {
                continue; // aborted/empty connection: skip, keep serving
            }
            let head = String::from_utf8_lossy(&data[..head_end]).to_string();
            let content_length = head
                .to_ascii_lowercase()
                .lines()
                .find(|l| l.starts_with("content-length:"))
                .and_then(|l| l.split(':').nth(1))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            let mut body_remaining = content_length.saturating_sub(data.len() - head_end);
            let mut aborted = false;
            while body_remaining > 0 {
                let Ok(n) = stream.read(&mut chunk) else {
                    aborted = true;
                    break;
                };
                if n == 0 {
                    aborted = true;
                    break;
                }
                body_remaining = body_remaining.saturating_sub(n);
            }
            if aborted {
                continue;
            }

            let request_line = head.lines().next().unwrap_or_default().to_string();
            let is_login = request_line.starts_with("POST");
            let is_secret_get = request_line.starts_with("GET ");
            recorded.lock().unwrap().push(request_line);

            let body = if is_login {
                r#"{"access_token":"fake-token","expires_in":300,"token_type":"Bearer"}"#
            } else {
                r#"{"secret":{"secretKey":"WORKOS_CLIENT_SECRET","secretValue":"from-fake-server"}}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
            if is_secret_get {
                return; // the test's two requests have both been served
            }
        }
    });

    (format!("http://{addr}"), requests)
}

#[test]
fn production_fetch_uses_official_v4_named_secret_route() {
    let (base_url, requests) = fake_infisical_server();
    let config = InfisicalConfig {
        client_id: "sa-id".into(),
        client_secret: "sa-secret".into(),
        project_id: "test-project".into(),
        base_url,
    };

    let value = fetch_via_infisical(
        config,
        WORKOS_CLIENT_SECRET_KEY,
        WORKOS_INFISICAL_FOLDER,
        "dev",
    )
    .expect("fake server must serve the secret");
    assert_eq!(value, "from-fake-server");

    let recorded = requests.lock().unwrap();
    assert_eq!(recorded.len(), 2, "login + get: {}", recorded.join(" | "));
    assert!(
        recorded[0].starts_with("POST /api/v1/auth/universal-auth/login"),
        "unexpected login route: {}",
        recorded[0]
    );
    let get = &recorded[1];
    assert!(
        get.starts_with("GET /api/v4/secrets/WORKOS_CLIENT_SECRET?"),
        "unexpected secret route: {get}"
    );
    assert!(get.contains("projectId=test-project"), "{get}");
    assert!(!get.contains("project_id="), "legacy query key: {get}");
    assert!(get.contains("environment=dev"), "{get}");
    assert!(get.contains("secretPath=%2Fshared%2Fworkos"), "{get}");
    assert!(get.contains("type=shared"), "{get}");
    assert!(get.contains("viewSecretValue=true"), "{get}");
}

/// Fake that answers a valid universal-auth login but a non-success v4 read
/// carrying a unique body marker. The marker must never surface in the error
/// returned by `get_secret_by_name` (status/category-only diagnostics).
fn failing_infisical_server() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    std::thread::spawn(move || {
        for _ in 0..2 {
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
            let head = String::from_utf8_lossy(&data[..head_end]).to_string();
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

            let is_login = head.lines().next().unwrap_or_default().starts_with("POST");
            let (status, body) = if is_login {
                (
                    "200 OK",
                    r#"{"access_token":"fake-token","expires_in":300,"token_type":"Bearer"}"#,
                )
            } else {
                (
                    "500 Internal Server Error",
                    r#"{"marker":"BODY-MARKER-NEVER-LEAK-7f3a"}"#,
                )
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    format!("http://{addr}")
}

#[test]
fn v4_non_success_error_excludes_response_body() {
    let config = InfisicalConfig {
        client_id: "sa-id".into(),
        client_secret: "sa-secret".into(),
        project_id: "test-project".into(),
        base_url: failing_infisical_server(),
    };
    let mut client = InfisicalClient::new(config);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let err = runtime
        .block_on(client.get_secret_by_name(
            WORKOS_CLIENT_SECRET_KEY,
            WORKOS_INFISICAL_FOLDER,
            "dev",
        ))
        .expect_err("non-success status must be an error");

    let rendered = format!("{err} | {err:?}");
    assert!(
        !rendered.contains("BODY-MARKER-NEVER-LEAK-7f3a"),
        "response body leaked into error: {rendered}"
    );
    assert!(
        rendered.contains("500"),
        "status must still be reported: {rendered}"
    );
}
