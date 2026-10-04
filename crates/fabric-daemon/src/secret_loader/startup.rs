//! Startup seam: base-URL gating plus env/config-first secret resolution.
//!
//! Split out of `mod.rs` to keep that file within the repository line limit.
//! This is the single production entry point `cmd_start` calls, so tests
//! exercise the rejected-base-URL branch rather than copying it.

use super::{
    base_url_rejection_warning, load_workos_client_secret, resolve_infisical_base_url,
    resolve_preconfigured, BaseUrlRejection, FetchFailure, LoadOutcome, Source,
};
use crate::config::AuthConfig;

/// Fetcher signature for the startup seam: receives the resolved base URL
/// first, so the seam owns base-URL resolution and the caller does not.
type BaseUrlFetch<'a> = &'a mut dyn FnMut(&str, &str, &str, &str) -> Result<String, FetchFailure>;

/// Resolve the startup WorkOS client secret across the base-URL gate.
///
/// `fetch` receives the resolved base URL first, so the seam owns base-URL
/// resolution and the caller never pre-resolves it.
///
/// A rejected `INFISICAL_BASE_URL` override skips the fetch but still resolves
/// preconfigured sources, so an operator with `WORKOS_CLIENT_SECRET` already in
/// the environment or config keeps that source instead of being reported unset
/// (review finding, 2026-10-04). The rejection is attached as a warning only
/// when nothing was preconfigured. Returns the outcome plus any rejection for
/// the caller to report after logging init.
pub fn resolve_startup_workos_secret(
    auth: &mut AuthConfig,
    read_env: &dyn Fn(&str) -> Option<String>,
    fetch: BaseUrlFetch<'_>,
) -> (LoadOutcome, Option<BaseUrlRejection>) {
    match resolve_infisical_base_url(read_env) {
        Ok(base_url) => {
            let outcome =
                load_workos_client_secret(auth, read_env, &mut |s, f, e| fetch(&base_url, s, f, e));
            (outcome, None)
        }
        Err(rejection) => {
            let outcome = match resolve_preconfigured(auth, read_env) {
                Some(source) => LoadOutcome {
                    source,
                    warning: None,
                },
                None => LoadOutcome {
                    source: Source::Unset,
                    warning: Some(base_url_rejection_warning(rejection)),
                },
            };
            (outcome, Some(rejection))
        }
    }
}
