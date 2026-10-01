//! Review-response tests for the #20 bot findings (Kilo Code, CodeAnt,
//! 2026-09-29).
//!
//! Every test here was captured RED before the production change. Grouped by
//! finding so a regression names the contract it broke:
//! - F1 a malformed credential-bearing base URL must fail CLOSED (no request
//!   may leave the host), not fail open to the public Infisical cloud;
//! - F2 embedded userinfo must be rejected, including the `host@evil.example`
//!   retarget that silently redirects a mistyped host;
//! - F3 the rejection must be reportable after logging init, without the value;
//! - F4 IPv6 hosts must keep their brackets;
//! - F5 a host literally named `api` must not be mangled by suffix stripping;
//! - F6 a rejected token must not stay cached and poison later fetches;
//! - F7 no documented diagnostic category may be unreachable.

use super::tests::reader;
use super::*;

use crate::auth::SecretValue;

use std::cell::RefCell;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Status-only fake: answers universal-auth login with a token, then serves
/// the v4 read with `read_status` for the first `bad_reads` GETs and a valid
/// secret afterwards. Records GET count for retry assertions.
///
/// F8: unlike the #20 fake, a malformed request head is answered with `400`
/// instead of a silent close, and the accept loop is bounded by an expected
/// request count rather than a fixed 6, so a regression fails as a status
/// mismatch instead of a 30s client timeout.
fn flaky_auth_server(read_status: &'static str, bad_reads: usize) -> (String, Arc<AtomicUsize>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let gets: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let recorded = Arc::clone(&gets);
    // Each attempt costs a login + a read. `bad_reads` failures plus one
    // recovery, and the recovery re-authenticates (F6), so budget two
    // connections per attempt: a too-small budget shows up as a transport
    // error rather than a silent hang.
    let expected = 2 * (bad_reads + 1);

    std::thread::spawn(move || {
        for _ in 0..expected {
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
            // F8: answer a malformed head rather than closing silently.
            if head_end == 0 {
                let _ = stream.write_all(
                    b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
                let _ = stream.flush();
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
                if n < bad_reads {
                    (
                        read_status.to_string(),
                        r#"{"marker":"NEVER-LEAK"}"#.to_string(),
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

fn auth_ready() -> crate::config::AuthConfig {
    crate::config::AuthConfig {
        enabled: true,
        workos_client_id: "client_abc".into(),
        workos_client_secret: String::new(),
        infisical_client_id: "sa-id".into(),
        infisical_client_secret: "sa-secret".into(),
        infisical_project_id: "proj".into(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------- F1: fail closed

/// F1: a malformed override must NOT silently retarget the service-account
/// client secret at the public cloud. `Err` means "contact no host".
#[test]
fn malformed_base_url_fails_closed_instead_of_defaulting_to_public_cloud() {
    let default_url = crate::auth::InfisicalConfig::default().base_url;
    for bad in [
        "eu.infisical.com",
        "https://eu.infisical.com/api/v1",
        "ftp://eu.infisical.com",
        "https://infisical.corp.example/infisical",
    ] {
        let read = reader(&[(ENV_INFISICAL_BASE_URL, bad)]);
        let resolved = resolve_infisical_base_url(&read);
        assert!(
            resolved.is_err(),
            "malformed override {bad} must fail closed, got {resolved:?} \
             (returning a default would POST the service-account client secret \
             to a host the operator never configured)"
        );
        assert!(
            !format!("{resolved:?}").contains(&default_url),
            "rejection must not carry the public default"
        );
    }
}

/// F1: an absent or whitespace-only override is the normal unset case and must
/// still resolve to the documented default.
#[test]
fn absent_or_blank_base_url_still_uses_documented_default() {
    let default_url = crate::auth::InfisicalConfig::default().base_url;
    for raw in ["", "   ", "\t\n"] {
        let read = reader(&[(ENV_INFISICAL_BASE_URL, raw)]);
        assert_eq!(
            resolve_infisical_base_url(&read).as_deref().ok(),
            Some(default_url.as_str()),
            "blank override {raw:?} is the unset case"
        );
    }
    let empty = reader(&[]);
    assert_eq!(
        resolve_infisical_base_url(&empty).as_deref().ok(),
        Some(default_url.as_str()),
        "an absent override must use the documented default"
    );
}

/// F1 (the live leak): with a rejected base URL the loader must attempt no
/// Infisical request at all. Before the fix this recorded an actual fetch
/// against the public default.
#[test]
fn rejected_base_url_suppresses_the_infisical_fetch_entirely() {
    let calls: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let mut auth = auth_ready();
    let read = reader(&[(ENV_INFISICAL_BASE_URL, "not a url")]);
    let base = resolve_infisical_base_url(&read);

    let outcome = match base {
        Ok(_base_url) => {
            load_workos_client_secret(&mut auth, &reader(&[]), &mut |name, folder, env| {
                calls.borrow_mut().push(format!("{name}@{folder}:{env}"));
                Ok("from-fake".to_string())
            })
        }
        Err(rejection) => LoadOutcome {
            source: Source::Unset,
            warning: Some(base_url_rejection_warning(rejection)),
        },
    };

    assert!(
        calls.borrow().is_empty(),
        "no Infisical request may be attempted when the base URL is rejected, got {:?}",
        calls.borrow()
    );
    assert_eq!(outcome.source, Source::Unset);
    assert_eq!(
        outcome.warning.as_ref().map(|w| w.category),
        Some("unparseable"),
        "operator must see a category, and the fallback must stay unset"
    );
    assert!(
        auth.workos_client_secret.is_empty(),
        "no secret may be stored when the fallback is skipped"
    );
}

// ---------------------------------------------------------------- F2: userinfo

/// F2: embedded userinfo is credential-bearing and confusable. Both the
/// "silently discarded credentials" form and the `host@evil.example` retarget
/// must be rejected outright.
#[test]
fn base_url_with_embedded_userinfo_is_rejected() {
    for bad in [
        "https://user:pass@infisical.corp.example",
        "https://infisical.corp.example@evil.example",
        "https://token@infisical.corp.example",
    ] {
        assert_eq!(
            normalize_infisical_base_url(bad),
            Err(BaseUrlRejection::UserInfo),
            "userinfo-bearing value must be rejected, not silently stripped: {bad}"
        );
    }
}

// ---------------------------------------------------------------- F3: reporting

/// F3: the rejection must be reportable where logging is live, carry no part
/// of the offending value, and name the variable so it can be fixed.
#[test]
fn base_url_rejection_is_reportable_after_logging_init() {
    let read = reader(&[(
        ENV_INFISICAL_BASE_URL,
        "https://infisical.corp.example@evil.example",
    )]);
    let err = resolve_infisical_base_url(&read).expect_err("must fail closed");
    let rendered = format!("{err:?}");

    assert!(
        !rendered.contains("evil.example"),
        "the rejected value must never appear in the report: {rendered}"
    );
    assert!(
        !rendered.contains("corp.example"),
        "no part of the rejected host may appear: {rendered}"
    );

    let warning = base_url_rejection_warning(err);
    let rendered_warning = format!("{warning}");
    assert!(
        !rendered_warning.contains("evil.example"),
        "the warning must stay redaction-safe: {rendered_warning}"
    );
    assert!(!warning.category.is_empty(), "a category must be reported");
}

// ---------------------------------------------------------------- F4: IPv6

/// F4: `host_str()` drops IPv6 brackets, producing an unparseable origin.
#[test]
fn ipv6_base_url_keeps_brackets() {
    assert_eq!(
        normalize_infisical_base_url("http://[::1]:8080"),
        Ok("http://[::1]:8080".into())
    );
    assert_eq!(
        normalize_infisical_base_url("https://[2001:db8::1]"),
        Ok("https://[2001:db8::1]".into())
    );
    // A bracketed IPv6 host must round-trip through the real client, or the
    // rebuilt origin is unparseable where it is actually used.
    let (base_url, _gets) = flaky_auth_server("200 OK", 0);
    let parsed = reqwest::Url::parse(&base_url).expect("fake base url parses");
    let port = parsed.port().unwrap();
    let rebuilt = format!("http://[::1]:{port}");
    assert!(
        reqwest::Url::parse(&rebuilt).is_ok(),
        "{rebuilt} must reparse"
    );
}

// ---------------------------------------------------------------- F5: `api` host

/// F5: stripping `/api` must only apply to the parsed path, so a host
/// literally named `api` survives unchanged.
#[test]
fn host_literally_named_api_is_not_mangled() {
    assert_eq!(
        normalize_infisical_base_url("https://api"),
        Ok("https://api".into())
    );
    assert_eq!(
        normalize_infisical_base_url("https://api:8443"),
        Ok("https://api:8443".into())
    );
    // ...while the documented CLI trailing `/api` is still stripped.
    assert_eq!(
        normalize_infisical_base_url("https://eu.infisical.com/api"),
        Ok("https://eu.infisical.com".into())
    );
}

// ---------------------------------------------------------------- F6: token cache

/// F6: a rejected token must be evicted, so the next attempt re-authenticates
/// instead of replaying a token the server already refused.
#[test]
fn rejected_token_is_evicted_so_a_later_fetch_reauthenticates() {
    let (base_url, gets) = flaky_auth_server("401 Unauthorized", 1);
    let mut client = client_for(base_url);

    let err = block_on_read(&mut client).expect_err("first read must fail");
    assert!(matches!(err, SecretsError::Auth(_)), "got {err}");

    let second = block_on_read(&mut client).expect("second read must recover");
    assert_eq!(second.value, "from-fake-server");
    assert_eq!(
        gets.load(Ordering::SeqCst),
        2,
        "the second read must be a fresh authenticated attempt, not a replay \
         of the rejected token"
    );
}

/// F6 companion: a 403 (valid token, secret not permitted) must also evict, so
/// a later permission change is not masked by a poisoned cache.
#[test]
fn forbidden_token_is_evicted_so_a_later_fetch_reauthenticates() {
    let (base_url, gets) = flaky_auth_server("403 Forbidden", 1);
    let mut client = client_for(base_url);

    let err = block_on_read(&mut client).expect_err("first read must fail");
    assert!(matches!(err, SecretsError::Auth(_)), "got {err}");

    let second = block_on_read(&mut client).expect("second read must recover");
    assert_eq!(second.value, "from-fake-server");
    assert_eq!(gets.load(Ordering::SeqCst), 2);
}

// ---------------------------------------------------------------- F7: reachable categories

/// F7: the documented `serialization` category must be constructible, and the
/// runbook's category list must match the reachable enum exactly.
#[test]
fn every_documented_category_is_constructible() {
    assert_eq!(
        FetchFailure::from(&SecretsError::Serialization("x".into())).category(),
        "serialization"
    );

    let runbook = include_str!("../../../../docs/runbooks/workos-infisical-shared.md");
    let documented: std::collections::BTreeSet<&str> = [
        "unavailable",
        "auth",
        "not_found",
        "operation",
        "serialization",
        "empty_response",
    ]
    .into_iter()
    .collect();
    for category in &documented {
        assert!(
            runbook.contains(category),
            "runbook must document the reachable category `{category}`"
        );
    }

    let actual: std::collections::BTreeSet<&str> = [
        FetchFailure::Unavailable,
        FetchFailure::Auth,
        FetchFailure::NotFound,
        FetchFailure::Operation,
        FetchFailure::Serialization,
        FetchFailure::EmptyResponse,
    ]
    .into_iter()
    .map(FetchFailure::category)
    .collect();
    assert_eq!(
        documented, actual,
        "runbook categories must match the reachable enum exactly"
    );
}

/// F7: a malformed token body must surface as `serialization`, proving the
/// category is reachable rather than merely declared.
#[test]
fn malformed_token_body_maps_to_serialization_category() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    std::thread::spawn(move || {
        // Answer the login with a body that is not a TokenResponse.
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
            if head_end == 0 {
                continue;
            }
            let head = String::from_utf8_lossy(&data[..head_end]).to_string();
            let body = if head.lines().next().unwrap_or_default().starts_with("POST") {
                r#"{"unexpected":"shape"}"#
            } else {
                r#"{"secret":{"secretKey":"WORKOS_CLIENT_SECRET","secretValue":"x"}}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    let mut client = client_for(format!("http://{addr}"));
    let err = block_on_read(&mut client).expect_err("malformed token body must fail");
    assert!(
        matches!(err, SecretsError::Serialization(_)),
        "a malformed token body must map to Serialization, got {err:?}"
    );
    assert_eq!(FetchFailure::from(&err).category(), "serialization");
}
