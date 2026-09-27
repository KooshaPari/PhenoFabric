//! Tests for the secret loader. See `super` for the production code.
//!
//! All tests are deterministic and local: the env reader and Infisical
//! fetcher are injected, and the production HTTP adapter is exercised only
//! against an in-process fake server bound to 127.0.0.1.

use super::*;

use std::cell::RefCell;
use std::collections::HashMap;

type RecordedCalls = Vec<(String, String, String)>;

fn record(calls: &RefCell<RecordedCalls>, name: &str, folder: &str, env: &str) {
    calls
        .borrow_mut()
        .push((name.to_string(), folder.to_string(), env.to_string()));
}

fn expected_call(name: &str, folder: &str, env: &str) -> (String, String, String) {
    (name.to_string(), folder.to_string(), env.to_string())
}

pub(super) fn reader(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |name: &str| map.get(name).cloned()
}

pub(super) fn auth_with_creds() -> AuthConfig {
    // Fetch prerequisites per the runbook contract: auth enabled, a
    // WorkOS client ID configured, and the Infisical trio present.
    AuthConfig {
        enabled: true,
        workos_client_id: "client_test".into(),
        infisical_client_id: "sa-id".into(),
        infisical_client_secret: "sa-secret".into(),
        infisical_project_id: "test-project".into(),
        ..Default::default()
    }
}

#[test]
fn nonempty_env_secret_wins_and_skips_fetch() {
    let mut auth = auth_with_creds();
    let read = reader(&[(ENV_WORKOS_CLIENT_SECRET, "from-env")]);
    let calls: RefCell<RecordedCalls> = RefCell::new(Vec::new());
    let mut fetch = |name: &str, folder: &str, env: &str| {
        record(&calls, name, folder, env);
        Ok::<_, FetchFailure>("from-infisical".to_string())
    };

    let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);

    assert_eq!(auth.workos_client_secret, "from-env");
    assert_eq!(outcome.source, Source::Environment);
    assert!(outcome.warning.is_none());
    assert!(
        calls.borrow().is_empty(),
        "env value must skip remote fetch"
    );
}

#[test]
fn empty_env_secret_is_ignored_and_fallback_runs() {
    let mut auth = auth_with_creds();
    auth.workos_client_secret = String::new();
    let read = reader(&[(ENV_WORKOS_CLIENT_SECRET, "")]);
    let mut fetch = |_: &str, _: &str, _: &str| Ok::<_, FetchFailure>("from-infisical".to_string());

    let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);

    assert_eq!(auth.workos_client_secret, "from-infisical");
    assert_eq!(outcome.source, Source::Infisical);
    assert!(outcome.warning.is_none());
}

#[test]
fn missing_env_fetches_shared_workos_folder_default_dev() {
    let mut auth = auth_with_creds();
    let read = reader(&[]);
    let calls: RefCell<RecordedCalls> = RefCell::new(Vec::new());
    let mut fetch = |name: &str, folder: &str, env: &str| {
        record(&calls, name, folder, env);
        Ok::<_, FetchFailure>("from-infisical".to_string())
    };

    let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);

    assert_eq!(auth.workos_client_secret, "from-infisical");
    assert_eq!(outcome.source, Source::Infisical);
    assert_eq!(
        *calls.borrow(),
        vec![expected_call(
            WORKOS_CLIENT_SECRET_KEY,
            WORKOS_INFISICAL_FOLDER,
            "dev"
        )],
    );
}

#[test]
fn explicit_infisical_env_is_used() {
    let mut auth = auth_with_creds();
    let read = reader(&[(ENV_INFISICAL_ENV, "staging")]);
    let calls: RefCell<RecordedCalls> = RefCell::new(Vec::new());
    let mut fetch = |name: &str, folder: &str, env: &str| {
        record(&calls, name, folder, env);
        Ok::<_, FetchFailure>("from-infisical".to_string())
    };

    load_workos_client_secret(&mut auth, &read, &mut fetch);

    assert_eq!(calls.borrow()[0].2, "staging");
}

#[test]
fn empty_infisical_env_falls_back_to_dev() {
    let mut auth = auth_with_creds();
    let read = reader(&[(ENV_INFISICAL_ENV, "")]);
    let calls: RefCell<RecordedCalls> = RefCell::new(Vec::new());
    let mut fetch = |name: &str, folder: &str, env: &str| {
        record(&calls, name, folder, env);
        Ok::<_, FetchFailure>("from-infisical".to_string())
    };

    load_workos_client_secret(&mut auth, &read, &mut fetch);

    assert_eq!(calls.borrow()[0].2, "dev");
}

#[test]
fn loader_ignores_credential_env_vars_and_reads_config_only() {
    let mut auth = AuthConfig::default(); // all infisical_* config fields empty
                                          // Credential env vars are owned by `DaemonConfig::load_env_secrets`
                                          // (called by the startup wiring before this loader). The fallback itself must never
                                          // re-read them: even present here, they must not enable a fetch.
    let read = reader(&[
        ("INFISICAL_CLIENT_SECRET", "env-sa-secret"),
        ("INFISICAL_CLIENT_ID", "env-sa-id"),
        ("INFISICAL_PROJECT_ID", "env-project"),
    ]);
    let calls: RefCell<RecordedCalls> = RefCell::new(Vec::new());
    let mut fetch = |name: &str, folder: &str, env: &str| {
        record(&calls, name, folder, env);
        Ok::<_, FetchFailure>("from-infisical".to_string())
    };

    let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);

    assert!(calls.borrow().is_empty());
    assert_eq!(auth.workos_client_secret, "");
    assert_eq!(outcome.source, Source::Unset);
    assert!(outcome.warning.is_none());
}

#[test]
fn fetch_unavailable_is_nonfatal_with_redacted_warning() {
    let mut auth = auth_with_creds();
    auth.enabled = true; // required-secret config still must not abort here
    let read = reader(&[]);
    let mut fetch = |_: &str, _: &str, _: &str| Err(FetchFailure::Unavailable);

    let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);

    assert_eq!(auth.workos_client_secret, "");
    let warning = outcome.warning.expect("failure must be reported");
    assert_eq!(warning.secret, WORKOS_CLIENT_SECRET_KEY);
    assert_eq!(warning.folder, WORKOS_INFISICAL_FOLDER);
    assert_eq!(warning.environment, "dev");
    assert_eq!(warning.category, "unavailable");
}

#[test]
fn fetch_not_found_is_nonfatal_with_redacted_warning() {
    let mut auth = auth_with_creds();
    auth.enabled = true;
    let read = reader(&[]);
    let mut fetch = |_: &str, _: &str, _: &str| Err(FetchFailure::NotFound);

    let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);

    assert_eq!(auth.workos_client_secret, "");
    let warning = outcome.warning.expect("failure must be reported");
    assert_eq!(warning.category, "not_found");
}

#[test]
fn error_surface_is_category_only_and_never_leaks_bodies() {
    // A SecretsError embedding a third-party response body must map to a
    // category with no payload, and the warning must render only names.
    let leaky =
        SecretsError::Auth(r#"HTTP 401: {"access_token":"tok-should-never-appear"}"#.to_string());
    let failure = FetchFailure::from(&leaky);
    assert_eq!(failure.category(), "auth");

    let warning = FallbackWarning {
        secret: WORKOS_CLIENT_SECRET_KEY,
        folder: WORKOS_INFISICAL_FOLDER,
        environment: "dev".to_string(),
        category: failure.category(),
    };
    let rendered = format!("{warning} | {warning:?} | {}", failure.category());
    assert!(rendered.contains(WORKOS_CLIENT_SECRET_KEY));
    assert!(rendered.contains("auth"));
    assert!(!rendered.contains("access_token"));
    assert!(!rendered.contains("tok-should-never-appear"));
    assert!(!rendered.contains("HTTP 401"));
}

#[test]
fn non_secret_env_values_are_owned_by_merge_not_by_loader() {
    // Kept here as a negative: the loader must not move these; the sole
    // env-merge authority (`DaemonConfig::load_env_secrets` in config.rs)
    // owns them.
    let mut auth = AuthConfig {
        workos_client_id: "config-id".into(),
        ..Default::default()
    };
    let read = reader(&[
        ("WORKOS_CLIENT_ID", "env-id"), // must be ignored by the loader
        ("WORKOS_REDIRECT_URI", "http://localhost:5173/cb"),
        ("INFISICAL_CLIENT_SECRET", "env-sa-secret"),
    ]);
    let mut fetch = |_: &str, _: &str, _: &str| Ok::<_, FetchFailure>("unused".to_string());

    load_workos_client_secret(&mut auth, &read, &mut fetch);

    assert_eq!(auth.workos_client_id, "config-id");
    assert_eq!(auth.workos_redirect_uri, "");
    assert_eq!(auth.infisical_client_secret, "");
}

#[test]
fn config_file_secret_skips_fetch() {
    // A config-file value (surviving load_env_secrets) is preserved as-is:
    // the fetch only fills a missing secret, so it must not be attempted.
    let mut auth = auth_with_creds();
    auth.workos_client_secret = "from-config".into();
    let read = reader(&[]);
    let calls: RefCell<RecordedCalls> = RefCell::new(Vec::new());
    let mut fetch = |name: &str, folder: &str, env: &str| {
        record(&calls, name, folder, env);
        Ok::<_, FetchFailure>("from-infisical".to_string())
    };

    let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);

    assert_eq!(auth.workos_client_secret, "from-config");
    assert_eq!(outcome.source, Source::Config);
    assert!(outcome.warning.is_none());
    assert!(
        calls.borrow().is_empty(),
        "config value must skip remote fetch"
    );
}

#[test]
fn fetch_requires_auth_enabled_and_workos_client_id() {
    // Runbook contract: never attempt the fetch when the secret could not
    // be used anyway (auth disabled, or no WorkOS client ID configured).
    let read = reader(&[]);
    let calls: RefCell<RecordedCalls> = RefCell::new(Vec::new());
    let mut fetch = |name: &str, folder: &str, env: &str| {
        record(&calls, name, folder, env);
        Ok::<_, FetchFailure>("from-infisical".to_string())
    };

    let mut auth = auth_with_creds();
    auth.enabled = false;
    let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);
    assert!(calls.borrow().is_empty());
    assert_eq!(auth.workos_client_secret, "");
    assert_eq!(outcome.source, Source::Unset);
    assert!(outcome.warning.is_none());

    let mut auth = auth_with_creds();
    auth.workos_client_id.clear();
    let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);
    assert!(calls.borrow().is_empty());
    assert_eq!(auth.workos_client_secret, "");
    assert_eq!(outcome.source, Source::Unset);
    assert!(outcome.warning.is_none());
}

#[test]
fn partial_infisical_prerequisites_skip_fetch() {
    // "Fetch only when client ID, Infisical client secret, and project ID
    // are all nonempty": missing members must skip the fetch entirely.
    let mut auth = auth_with_creds();
    auth.infisical_client_secret.clear();
    auth.infisical_project_id.clear();
    let read = reader(&[]);
    let calls: RefCell<RecordedCalls> = RefCell::new(Vec::new());
    let mut fetch = |name: &str, folder: &str, env: &str| {
        record(&calls, name, folder, env);
        Ok::<_, FetchFailure>("from-infisical".to_string())
    };

    let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);

    assert!(calls.borrow().is_empty());
    assert_eq!(auth.workos_client_secret, "");
    assert_eq!(outcome.source, Source::Unset);
    assert!(outcome.warning.is_none());
}

#[test]
fn fetched_empty_or_whitespace_value_is_rejected() {
    // A successful HTTP response carrying an empty/whitespace secret must
    // not be stored; it surfaces as a redacted empty_response warning.
    for bad in ["", "   ", "\t\n"] {
        let mut auth = auth_with_creds();
        let read = reader(&[]);
        let value = bad.to_string();
        let mut fetch = move |_: &str, _: &str, _: &str| Ok::<_, FetchFailure>(value.clone());

        let outcome = load_workos_client_secret(&mut auth, &read, &mut fetch);

        assert_eq!(auth.workos_client_secret, "");
        assert_eq!(outcome.source, Source::Unset);
        let warning = outcome.warning.expect("empty fetch must warn");
        assert_eq!(warning.category, "empty_response");
    }
}
