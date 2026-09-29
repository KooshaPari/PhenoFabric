//! Startup wiring test: a secret loaded by the loader must flow into
//! `AuthMiddlewareConfig`, i.e. the daemon's existing auth
//! validation/consumption path. Env merging into config is owned and tested
//! by `DaemonConfig::load_env_secrets` in `config.rs`; the loader re-reads
//! only `WORKOS_CLIENT_SECRET` (source attribution) and `INFISICAL_ENV`.
//! This file only covers the loader-to-consumer wiring.

use super::tests::{auth_with_creds, reader};
use super::*;

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
        resolve_infisical_base_url(&read),
        "https://eu.infisical.com"
    );
}

#[test]
fn base_url_env_override_empty_or_absent_falls_back_to_us_cloud_default() {
    let absent = reader(&[]);
    let empty = reader(&[(ENV_INFISICAL_BASE_URL, "   ")]);
    let default_url = crate::auth::InfisicalConfig::default().base_url;
    assert_eq!(resolve_infisical_base_url(&absent), default_url);
    assert_eq!(resolve_infisical_base_url(&empty), default_url);
}
