//! Startup wiring test: a secret loaded by the loader must flow into
//! `AuthMiddlewareConfig`, i.e. the daemon's existing auth
//! validation/consumption path. Env merging into config is owned and tested
//! by `DaemonConfig::load_env_secrets` in `config.rs`; the loader re-reads
//! only `WORKOS_CLIENT_SECRET` (source attribution) and `INFISICAL_ENV`.
//! This file only covers the loader-to-consumer wiring.

use super::tests::{auth_with_creds, reader};
use super::*;

use std::cell::RefCell;

#[test]
fn fallback_populated_secret_reaches_auth_middleware_config() {
    // Success integration: the daemon converts AuthConfig into
    // AuthMiddlewareConfig; the Infisical-populated secret must be visible
    // there, where existing auth validation/consumption reads it.
    let mut auth = auth_with_creds();
    auth.enabled = true;
    auth.workos_client_id = "client_test".into();
    let read = reader(&[]);
    let mut fetch = |_: &str, _: &str, _: &str| Ok::<_, FetchFailure>("from-infisical".to_string());

    let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);
    assert_eq!(outcome.source, Source::Infisical);
    assert!(outcome.warning.is_none());

    let middleware: crate::auth::AuthMiddlewareConfig = auth.into();
    assert!(middleware.enabled);
    let workos = middleware
        .workos_config
        .expect("workos config must exist when client id is set");
    assert_eq!(workos.client_secret, "from-infisical");
}

#[test]
fn base_url_env_override_accepts_cli_domain_form() {
    let read = reader(&[(ENV_INFISICAL_BASE_URL, "https://eu.infisical.com/api/")]);
    assert_eq!(
        resolve_infisical_base_url(&read).unwrap_or_default(),
        "https://eu.infisical.com"
    );
}

#[test]
fn base_url_env_override_empty_or_absent_falls_back_to_us_cloud_default() {
    let absent = reader(&[]);
    let empty = reader(&[(ENV_INFISICAL_BASE_URL, "   ")]);
    let default_url = crate::auth::InfisicalConfig::default().base_url;
    assert_eq!(
        resolve_infisical_base_url(&absent).as_deref().ok(),
        Some(default_url.as_str())
    );
    assert_eq!(
        resolve_infisical_base_url(&empty).as_deref().ok(),
        Some(default_url.as_str())
    );
}

// ---------------------------------------------- real `cmd_start` seam (PR #20)

/// Fail-closed at the REAL startup seam. `resolve_startup_workos_secret` is the
/// exact function `cmd_start` calls (not a test-local reimplementation of the
/// resolve/gate/load wiring). The injected fetch records every attempt, so a
/// nonempty log means startup tried to contact a host and, in production, would
/// have POSTed the service-account client secret there.
#[test]
fn startup_seam_rejected_base_url_attempts_no_fetch() {
    let attempts: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let mut auth = auth_with_creds();
    let read = reader(&[(ENV_INFISICAL_BASE_URL, "not a url")]);

    let (outcome, rejection) = resolve_startup_workos_secret(
        &mut auth,
        &read,
        &mut |config, secret, folder, environment| {
            attempts.borrow_mut().push(format!(
                "{}:{secret}@{folder}:{environment}",
                config.base_url
            ));
            Ok("SHOULD-NOT-BE-FETCHED".to_string())
        },
    );

    assert!(
        attempts.borrow().is_empty(),
        "startup must attempt no Infisical network I/O when the base URL is \
         rejected, got {:?}",
        attempts.borrow()
    );
    assert_eq!(outcome.source, Source::Unset);
    assert_eq!(
        outcome.warning.as_ref().map(|w| w.category),
        Some("unparseable")
    );
    assert!(rejection.is_some());
    assert!(
        auth.workos_client_secret.is_empty(),
        "no secret may be stored when the fallback is skipped"
    );
}

/// Control for the fail-closed seam test: with a well-formed override the same
/// seam DOES attempt exactly one fetch, against the operator's origin (never
/// the public default). This proves the attempt counter above is not vacuous
/// and would catch a fail-open regression.
#[test]
fn startup_seam_valid_base_url_attempts_fetch_at_configured_origin() {
    let attempts: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let mut auth = auth_with_creds();
    let read = reader(&[(ENV_INFISICAL_BASE_URL, "https://eu.infisical.com")]);

    let (outcome, rejection) = resolve_startup_workos_secret(
        &mut auth,
        &read,
        &mut |config, secret, folder, environment| {
            attempts.borrow_mut().push(format!(
                "{}:{secret}@{folder}:{environment}",
                config.base_url
            ));
            Ok("from-eu".to_string())
        },
    );

    assert_eq!(attempts.borrow().len(), 1, "exactly one fetch is attempted");
    assert_eq!(
        attempts.borrow()[0],
        "https://eu.infisical.com:WORKOS_CLIENT_SECRET@/shared/workos:dev",
        "the fetch must target the configured origin, not the public default"
    );
    assert_eq!(outcome.source, Source::Infisical);
    assert!(rejection.is_none());
    assert_eq!(auth.workos_client_secret, "from-eu");
}
