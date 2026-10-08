//! Tests for the startup seam `resolve_startup_workos_secret`, the single
//! production entry point `cmd_start` calls. Kept separate from
//! `review_response` so both files stay within the line limit.

use super::review_response::auth_ready;
use super::tests::reader;
use super::*;

use std::cell::RefCell;

/// Finding B: a plaintext (`http`) non-loopback override is refused by the real
/// startup seam, and no cleartext request is attempted (review finding,
/// 2026-10-04).
#[test]
fn plaintext_non_loopback_override_is_refused_without_fetch() {
    let mut auth = auth_ready();
    let read = reader(&[(ENV_INFISICAL_BASE_URL, "http://infisical.internal.example")]);
    let calls = RefCell::new(0usize);

    let (outcome, rejection) =
        resolve_startup_workos_secret(&mut auth, &read, &mut |_, _, _, _| {
            *calls.borrow_mut() += 1;
            Ok("from-fake".to_string())
        });

    assert_eq!(rejection, Some(BaseUrlRejection::InsecureTransport));
    assert_eq!(
        *calls.borrow(),
        0,
        "no cleartext request may be attempted against a non-loopback host"
    );
    assert_eq!(outcome.source, Source::Unset);
}

/// Finding D: a rejected override must not erase an already-set environment
/// secret. The source stays `Environment`, no fallback warning is produced, and
/// no request is attempted (review finding, 2026-10-04).
#[test]
fn rejected_base_url_preserves_environment_preconfigured_secret() {
    let mut auth = auth_ready();
    let read = reader(&[
        (ENV_INFISICAL_BASE_URL, "not a url"),
        ("WORKOS_CLIENT_SECRET", "env-secret"),
    ]);
    let calls = RefCell::new(0usize);

    let (outcome, rejection) =
        resolve_startup_workos_secret(&mut auth, &read, &mut |_, _, _, _| {
            *calls.borrow_mut() += 1;
            Ok("from-fake".to_string())
        });

    assert_eq!(rejection, Some(BaseUrlRejection::Unparseable));
    assert_eq!(outcome.source, Source::Environment);
    assert!(
        outcome.warning.is_none(),
        "a present secret is not a fallback failure"
    );
    assert_eq!(auth.workos_client_secret, "env-secret");
    assert_eq!(*calls.borrow(), 0);
}

/// Finding D (config form): the config-file secret survives a rejected override
/// with `Source::Config` rather than being reported unset.
#[test]
fn rejected_base_url_preserves_config_preconfigured_secret() {
    let mut auth = auth_ready();
    auth.workos_client_secret = "config-secret".into();
    let read = reader(&[(ENV_INFISICAL_BASE_URL, "not a url")]);
    let calls = RefCell::new(0usize);

    let (outcome, _rejection) =
        resolve_startup_workos_secret(&mut auth, &read, &mut |_, _, _, _| {
            *calls.borrow_mut() += 1;
            Ok("from-fake".to_string())
        });

    assert_eq!(outcome.source, Source::Config);
    assert!(outcome.warning.is_none());
    assert_eq!(auth.workos_client_secret, "config-secret");
    assert_eq!(*calls.borrow(), 0);
}
