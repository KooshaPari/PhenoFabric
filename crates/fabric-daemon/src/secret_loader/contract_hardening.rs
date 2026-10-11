use super::tests::reader;
use super::*;

use crate::auth::SecretValue;

use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Fake that answers a valid universal-auth login but rejects the v4 read
/// with a chosen status, then serves the secret on the next GET. Records the
/// number of GETs so a retry can be asserted.
fn unauthorized_then_ok_server(get_status: &'static str) -> (String, Arc<AtomicUsize>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let gets: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let recorded = Arc::clone(&gets);

    std::thread::spawn(move || {
        for _ in 0..6 {
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
            let (status, body): (String, String) = if is_login {
                (
                    "200 OK".to_string(),
                    r#"{"accessToken":"fake-token","expiresIn":300,"accessTokenMaxTTL":600,"tokenType":"Bearer"}"#
                        .to_string(),
                )
            } else {
                let n = recorded.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    // First read is rejected with the status under test.
                    (
                        get_status.to_string(),
                        format!(r#"{{"marker":"BODY-MARKER-NEVER-LEAK-7f3a-{get_status}"}}"#),
                    )
                } else {
                    (
                        "200 OK".to_string(),
                        r#"{"secret":{"secretKey":"WORKOS_CLIENT_SECRET","secretValue":"from-fake-server"}}"#
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

    (format!("http://{addr}"), gets)
}

fn client_for(base_url: String) -> InfisicalClient {
    InfisicalClient::new(InfisicalConfig {
        client_id: "sa-id".into(),
        client_secret: "sa-secret".into(),
        project_id: "test-project".into(),
        base_url,
    })
}

fn block_on_read(client: &mut InfisicalClient) -> Result<SecretValue, SecretsError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(client.get_secret_by_name(
        WORKOS_CLIENT_SECRET_KEY,
        WORKOS_INFISICAL_FOLDER,
        "dev",
    ))
}

/// A 401 on the v4 read means Infisical rejected the bearer token: the
/// category must be `auth`, not the generic `operation` (Kilo finding,
/// 2026-09-27). The response body must still never surface.
#[test]
fn v4_read_401_maps_to_auth_category_not_operation() {
    let (base_url, _gets) = unauthorized_then_ok_server("401 Unauthorized");
    let mut client = client_for(base_url);

    let err = block_on_read(&mut client).expect_err("401 must be an error");
    let rendered = format!("{err} | {err:?}");

    assert!(
        matches!(err, SecretsError::Auth(_)),
        "401 must map to SecretsError::Auth, got: {rendered}"
    );
    assert!(
        !rendered.contains("BODY-MARKER-NEVER-LEAK"),
        "response body leaked into error: {rendered}"
    );
}

/// 403 is the same failure mode (token valid, secret not permitted) and must
/// not be reported as an opaque operation failure.
#[test]
fn v4_read_403_maps_to_auth_category() {
    let (base_url, _gets) = unauthorized_then_ok_server("403 Forbidden");
    let mut client = client_for(base_url);

    let err = block_on_read(&mut client).expect_err("403 must be an error");
    assert!(
        matches!(err, SecretsError::Auth(_)),
        "403 must map to SecretsError::Auth, got: {err}"
    );
}

/// The `From<&SecretsError>` bridge is what the startup warning prints, so
/// the user-visible category must be `auth` for a rejected token.
#[test]
fn rejected_token_surfaces_auth_category_to_startup_warning() {
    let (base_url, _gets) = unauthorized_then_ok_server("401 Unauthorized");
    let mut client = client_for(base_url);

    let err = block_on_read(&mut client).expect_err("401 must be an error");
    let failure = FetchFailure::from(&err);

    assert_eq!(failure.category(), "auth");
    assert_ne!(
        failure.category(),
        "operation",
        "a rejected token must not be indistinguishable from a generic API error"
    );
}

/// No retry on 401: the cached token is not re-issued, so exactly one read
/// request must reach the API (retrying would double the credential exposure
/// for no gain).
#[test]
fn v4_read_401_does_not_silently_retry() {
    let (base_url, gets) = unauthorized_then_ok_server("401 Unauthorized");
    let mut client = client_for(base_url);

    let _ = block_on_read(&mut client).expect_err("401 must be an error");

    assert_eq!(
        gets.load(Ordering::SeqCst),
        1,
        "rejected token must not trigger a blind re-read"
    );
}

/// A 404 is still a distinct, actionable category (folder/environment/secret
/// name issue) and must not be folded into `auth`.
#[test]
fn v4_read_404_still_maps_to_not_found() {
    let (base_url, _gets) = unauthorized_then_ok_server("404 Not Found");
    let mut client = client_for(base_url);

    let err = block_on_read(&mut client).expect_err("404 must be an error");
    assert!(
        matches!(err, SecretsError::NotFound(_)),
        "404 must map to SecretsError::NotFound, got: {err}"
    );
    assert_eq!(FetchFailure::from(&err).category(), "not_found");
}

/// INFISICAL_BASE_URL is a credential-bearing destination: a typo must not
/// silently point the universal-auth POST at an arbitrary host, and must not
/// fail later inside reqwest as an opaque transport error.
#[test]
fn base_url_without_scheme_is_rejected_to_default() {
    assert_eq!(
        normalize_infisical_base_url("eu.infisical.com"),
        Err(BaseUrlRejection::Unparseable)
    );
    assert_eq!(
        normalize_infisical_base_url("app.infisical.com/api"),
        Err(BaseUrlRejection::Unparseable)
    );
    assert_eq!(
        normalize_infisical_base_url("//app.infisical.com"),
        Err(BaseUrlRejection::Unparseable)
    );
    assert_eq!(
        normalize_infisical_base_url("ftp://app.infisical.com"),
        Err(BaseUrlRejection::Scheme)
    );
    assert_eq!(
        normalize_infisical_base_url("not a url at all"),
        Err(BaseUrlRejection::Unparseable)
    );
}

/// A path beyond the documented origin form is rejected too: the client
/// appends `/api/v1/...` and `/api/v4/...` itself, so `/api/v1` in the
/// override produces `.../api/v1/api/v4/secrets/...`.
#[test]
fn base_url_with_unexpected_path_is_rejected_to_default() {
    assert_eq!(
        normalize_infisical_base_url("https://eu.infisical.com/api/v1"),
        Err(BaseUrlRejection::Path)
    );
    assert_eq!(
        normalize_infisical_base_url("https://eu.infisical.com/api/v4/secrets"),
        Err(BaseUrlRejection::Path)
    );
    assert_eq!(
        normalize_infisical_base_url("https://eu.infisical.com?a=1"),
        Err(BaseUrlRejection::Path)
    );
}

/// The documented forms still work unchanged: bare origin, trailing slash,
/// and the CLI `--domain` form with a trailing `/api`.
#[test]
fn base_url_documented_forms_still_accepted() {
    let expected = "https://eu.infisical.com";
    assert_eq!(normalize_infisical_base_url(expected), Ok(expected.into()));
    assert_eq!(
        normalize_infisical_base_url("https://eu.infisical.com/"),
        Ok(expected.into())
    );
    assert_eq!(
        normalize_infisical_base_url("https://eu.infisical.com/api"),
        Ok(expected.into())
    );
    assert_eq!(
        normalize_infisical_base_url("https://eu.infisical.com/api/"),
        Ok(expected.into())
    );
    assert_eq!(
        normalize_infisical_base_url("  https://eu.infisical.com/api  "),
        Ok(expected.into())
    );
    assert_eq!(
        normalize_infisical_base_url("http://127.0.0.1:8080"),
        Ok("http://127.0.0.1:8080".into())
    );
}

/// A rejected override fails closed (never defaults to the public host) and a
/// blank override still resolves to the documented default.
#[test]
fn resolve_fails_closed_for_malformed_override() {
    let default_url = crate::auth::InfisicalConfig::default().base_url;
    for bad in [
        "eu.infisical.com",
        "https://eu.infisical.com/api/v1",
        "ftp://eu.infisical.com",
    ] {
        let read = reader(&[(ENV_INFISICAL_BASE_URL, bad)]);
        assert!(
            resolve_infisical_base_url(&read).is_err(),
            "malformed override must fail closed: {bad}"
        );
    }
    for blank in ["", "   "] {
        let read = reader(&[(ENV_INFISICAL_BASE_URL, blank)]);
        assert_eq!(
            resolve_infisical_base_url(&read).as_deref().ok(),
            Some(default_url.as_str()),
            "blank override must use the documented default: {blank:?}"
        );
    }
}

/// A host that is a valid origin but obviously wrong is still accepted (we
/// cannot validate hostnames offline); the contract is origin-shape only, and
/// the runbook documents the expected form. This test pins that boundary so
/// a future validator does not silently start rejecting legitimate private
/// deployments.
#[test]
fn private_or_custom_origins_remain_accepted() {
    assert_eq!(
        normalize_infisical_base_url("https://infisical.internal.example"),
        Ok("https://infisical.internal.example".into())
    );
}
