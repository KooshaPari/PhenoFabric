//! Env-first secret loading with an Infisical fallback for `WORKOS_CLIENT_SECRET`.
//!
//! Two explicitly separated steps:
//! 1. [`crate::config::DaemonConfig::load_env_secrets`] — the sole env-merge
//!    authority for deployment values (`WORKOS_CLIENT_SECRET`,
//!    `WORKOS_CLIENT_ID`, `WORKOS_REDIRECT_URI`, `INFISICAL_CLIENT_SECRET`).
//!    Nonempty env values win over config; empty env values never erase
//!    config. It runs first, in `cmd_start`.
//! 2. [`load_workos_client_secret`] — the WorkOS client-secret resolver,
//!    in strict order: a nonempty `WORKOS_CLIENT_SECRET` process-env value
//!    wins and skips the network; a nonempty config-file value is then
//!    preserved as-is; only a missing secret triggers the Infisical fetch,
//!    and that fetch runs only when authentication is enabled, a WorkOS
//!    client ID is configured, and the Infisical client ID, client secret,
//!    and project ID are all nonempty — prerequisites read **only** from
//!    `AuthConfig` (already env-merged by step 1). The loader re-reads
//!    exactly two env vars from the process: `WORKOS_CLIENT_SECRET`, only
//!    to attribute `Source::Environment` (same nonempty-wins rule as step
//!    1 — the two must stay in lockstep), and `INFISICAL_ENV` for the
//!    fetch path; it never re-reads the Infisical service-account
//!    credentials. `INFISICAL_ENV` defaults to `dev` in the fetch path only.
//!    The fetch goes through
//!    [`fetch_via_infisical`], a thin blocking adapter over
//!    [`InfisicalClient::get_secret_by_name`] — the official
//!    `GET /api/v4/secrets/{secretName}` read (docs observed 2026-09-24,
//!    https://infisical.com/docs/api-reference/endpoints/secrets/read).
//!
//! The fallback is deliberately non-fatal: a failed or empty fetch yields a
//! redacted [`FallbackWarning`] (secret name, folder, environment, error
//! category only — never a secret value, response body, or token) and
//! startup continues so the existing auth validation remains the single
//! authority on requiring a usable WorkOS secret. A fetched empty or
//! whitespace-only value is rejected rather than stored.
//!
//! Tests inject the env reader and the fetcher (no process-env mutation, no
//! network); the production fetch adapter is exercised only against a local
//! in-process HTTP fake.

use crate::auth::{InfisicalClient, InfisicalConfig, SecretsError};
use crate::config::AuthConfig;

use std::fmt;

/// Secret key fetched from the shared WorkOS folder.
pub const WORKOS_CLIENT_SECRET_KEY: &str = "WORKOS_CLIENT_SECRET";
/// Infisical folder holding the shared WorkOS credential set.
pub const WORKOS_INFISICAL_FOLDER: &str = "/shared/workos";
/// Environment used when `INFISICAL_ENV` is unset or empty.
pub const DEFAULT_INFISICAL_ENV: &str = "dev";

const ENV_WORKOS_CLIENT_SECRET: &str = "WORKOS_CLIENT_SECRET";
const ENV_INFISICAL_ENV: &str = "INFISICAL_ENV";
/// Optional non-secret Infisical API base-URL override (US/EU/self-hosted).
/// Accepts a bare origin (`https://eu.infisical.com`) or the Infisical CLI
/// domain form with a trailing `/api`. Resolved in `cmd_start` before the
/// client is built; never persisted to TOML or returned over config IPC.
pub const ENV_INFISICAL_BASE_URL: &str = "INFISICAL_BASE_URL";

/// Redacted category of a fallback failure.
///
/// Carries no payload by construction: it is impossible to smuggle a
/// response body, token, or secret value through this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchFailure {
    /// Transport-level failure (connect/timeout/TLS).
    Unavailable,
    /// Infisical rejected the service-account authentication.
    Auth,
    /// The secret does not exist at that folder/environment.
    NotFound,
    /// The Infisical API returned a non-success status.
    Operation,
    /// The Infisical response could not be parsed.
    Serialization,
    /// The API responded successfully but with an empty secret value.
    EmptyResponse,
}

impl FetchFailure {
    /// Stable, log-safe category label.
    pub const fn category(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::Auth => "auth",
            Self::NotFound => "not_found",
            Self::Operation => "operation",
            Self::Serialization => "serialization",
            Self::EmptyResponse => "empty_response",
        }
    }
}

impl From<&SecretsError> for FetchFailure {
    fn from(err: &SecretsError) -> Self {
        match err {
            SecretsError::Http(_) => Self::Unavailable,
            SecretsError::Auth(_) => Self::Auth,
            SecretsError::NotFound(_) => Self::NotFound,
            SecretsError::OperationFailed(_) => Self::Operation,
            SecretsError::Serialization(_) => Self::Serialization,
        }
    }
}

/// Structured, redacted warning for a failed Infisical fallback.
///
/// Deliberately holds only names/categories; it can never contain a secret
/// value, response body, or token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackWarning {
    /// Secret key that could not be fetched.
    pub secret: &'static str,
    /// Infisical folder that was queried.
    pub folder: &'static str,
    /// Infisical environment slug that was queried.
    pub environment: String,
    /// Redacted error category (see [`FetchFailure::category`]).
    pub category: &'static str,
}

impl fmt::Display for FallbackWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (folder={}, env={}): {}",
            self.secret, self.folder, self.environment, self.category
        )
    }
}

/// Where the WorkOS client secret ultimately came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Nonempty environment value.
    Environment,
    /// Value already present in `AuthConfig` (config file or earlier
    /// merge): no environment value applied, and no fetch was needed
    /// because the fetch only fills a missing secret.
    Config,
    /// Fetched from Infisical by the fallback.
    Infisical,
    /// No value available.
    Unset,
}

impl Source {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::Config => "config",
            Self::Infisical => "infisical",
            Self::Unset => "unset",
        }
    }
}

/// Result of one secret-loading pass.
#[derive(Debug, PartialEq, Eq)]
pub struct LoadOutcome {
    /// Where the secret value came from.
    pub source: Source,
    /// Present only when the Infisical fallback was attempted and failed.
    pub warning: Option<FallbackWarning>,
}

/// Fetcher signature used by the loader seam.
///
/// Arguments: secret key name, folder path, environment slug.
type SecretFetch<'a> = &'a mut dyn FnMut(&str, &str, &str) -> Result<String, FetchFailure>;

/// Load the WorkOS client secret env-first, falling back to Infisical.
///
/// Resolution order: nonempty process env, then config-file value, then
/// the Infisical fetch (gated on enabled auth + configured WorkOS client
/// ID + the three Infisical prerequisites).
///
/// `read_env` is injected (never `std::env` directly) so tests are
/// deterministic; production passes `std::env::var(..).ok()`. Only
/// `WORKOS_CLIENT_SECRET` and `INFISICAL_ENV` are consulted from the
/// environment here — credential env vars are owned by
/// [`crate::config::DaemonConfig::load_env_secrets`], which runs first.
/// `fetch` is the Infisical seam; production wires
/// [`fetch_via_infisical`]. Never panics and never returns an error:
/// failures surface as [`LoadOutcome::warning`], and a fetched
/// empty/whitespace value is rejected without being stored.
pub fn load_workos_client_secret(
    auth: &mut AuthConfig,
    read_env: &dyn Fn(&str) -> Option<String>,
    fetch: SecretFetch<'_>,
) -> LoadOutcome {
    // 1. A nonempty, non-whitespace-only process-env value wins and skips
    //    the remote fetch; a whitespace-only value cannot authenticate, so
    //    it falls through instead of blocking the config/Infisical steps.
    if let Some(value) = read_env(ENV_WORKOS_CLIENT_SECRET) {
        if !value.trim().is_empty() {
            auth.workos_client_secret = value;
            return LoadOutcome {
                source: Source::Environment,
                warning: None,
            };
        }
    }

    // 2. A config-file value (surviving the step-1 merge) is preserved;
    //    the fetch only fills a missing secret.
    if !auth.workos_client_secret.is_empty() {
        return LoadOutcome {
            source: Source::Config,
            warning: None,
        };
    }

    // 3. Fetch only when authentication is enabled, a WorkOS client ID is
    //    configured, and all three Infisical prerequisites are nonempty
    //    (runbook contract: never fetch a secret that cannot be used yet).
    //    The value above is empty here, so a failed or rejected fetch
    //    below reports `Source::Unset`.
    let prerequisites_met = auth.enabled
        && !auth.workos_client_id.is_empty()
        && !auth.infisical_client_id.is_empty()
        && !auth.infisical_client_secret.is_empty()
        && !auth.infisical_project_id.is_empty();
    if !prerequisites_met {
        return LoadOutcome {
            source: Source::Unset,
            warning: None,
        };
    }

    // 4. INFISICAL_ENV defaults to `dev` in the fetch path only.
    let environment = match read_env(ENV_INFISICAL_ENV) {
        Some(value) if !value.is_empty() => value,
        _ => DEFAULT_INFISICAL_ENV.to_string(),
    };

    match fetch(
        WORKOS_CLIENT_SECRET_KEY,
        WORKOS_INFISICAL_FOLDER,
        &environment,
    ) {
        // Reject a fetched empty/whitespace value; never store it.
        Ok(value) if value.trim().is_empty() => LoadOutcome {
            source: Source::Unset,
            warning: Some(fallback_warning(
                &environment,
                FetchFailure::EmptyResponse.category(),
            )),
        },
        Ok(value) => {
            auth.workos_client_secret = value;
            LoadOutcome {
                source: Source::Infisical,
                warning: None,
            }
        }
        Err(failure) => LoadOutcome {
            source: Source::Unset,
            warning: Some(fallback_warning(&environment, failure.category())),
        },
    }
}

fn fallback_warning(environment: &str, category: &'static str) -> FallbackWarning {
    FallbackWarning {
        secret: WORKOS_CLIENT_SECRET_KEY,
        folder: WORKOS_INFISICAL_FOLDER,
        environment: environment.to_string(),
        category,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fetch_adapter;

#[cfg(test)]
mod contract_hardening;

#[cfg(test)]
mod env_wiring;

/// Normalize a configured Infisical base URL to a bare origin.
///
/// Accepts, per the Infisical CLI `--domain` contract (observed 2026-09-24):
/// `https://<host>`, the same with a trailing slash, and the CLI's domain
/// form with a trailing `/api` — the client appends `/api/v1/...` and
/// `/api/v4/...` itself.
///
/// The value is credential-bearing: the universal-auth login POST sends the
/// service-account client secret to whatever origin resolves here. Anything
/// that is not a clean http/https origin is therefore rejected (`None`) so
/// [`resolve_infisical_base_url`] falls back to the documented default and
/// warns, instead of failing later inside reqwest as an opaque transport
/// error or silently pointing the credential at a mistyped host.
///
/// Hostnames are not validated offline (a valid-but-wrong private host is a
/// legitimate deployment); only the origin *shape* is enforced.
pub fn normalize_infisical_base_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let without_api = trimmed.strip_suffix("/api").unwrap_or(trimmed);
    let normalized = without_api.trim_end_matches('/');
    if normalized.is_empty() {
        return None;
    }

    let url = reqwest::Url::parse(normalized).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    // A port is part of a legitimate origin; any path, query, or fragment is
    // not (the client supplies the full API path itself).
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return None;
    }
    if !url.has_host() {
        return None;
    }

    Some(match url.port() {
        Some(port) => format!("{}://{}:{port}", url.scheme(), url.host_str()?),
        None => format!("{}://{}", url.scheme(), url.host_str()?),
    })
}

/// Resolve the Infisical API base URL from an injected env reader, falling
/// back to the documented US Cloud default (`InfisicalConfig::default`).
/// Non-secret: read at startup only, never persisted or echoed over IPC.
///
/// A nonempty but malformed override is rejected and the default is used
/// instead, with a `tracing::warn!` (var name only, never the value) so the
/// misconfiguration is visible rather than silently ignored. Absent or
/// whitespace-only values are the normal unset case and stay silent.
pub fn resolve_infisical_base_url(read_env: &dyn Fn(&str) -> Option<String>) -> String {
    let default = crate::auth::InfisicalConfig::default().base_url;
    let Some(raw) = read_env(ENV_INFISICAL_BASE_URL).filter(|v| !v.trim().is_empty()) else {
        return default;
    };
    match normalize_infisical_base_url(&raw) {
        Some(url) => url,
        None => {
            tracing::warn!(
                variable = ENV_INFISICAL_BASE_URL,
                "infisical base url override rejected; expected https://<host> \
                 (optionally with the CLI trailing /api); using documented default"
            );
            default
        }
    }
}

/// Production fetch adapter: one blocking Infisical read via the existing
/// [`InfisicalClient::get_secret_by_name`] (official v4 named-secret read;
/// auth and HTTP logic stay in the client, nothing is duplicated here).
///
/// Builds a short-lived current-thread Tokio runtime; call only from sync
/// startup paths (not from within an async context).
pub fn fetch_via_infisical(
    config: InfisicalConfig,
    secret_name: &str,
    folder: &str,
    environment: &str,
) -> Result<String, FetchFailure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| FetchFailure::Unavailable)?;

    let result = runtime.block_on(async move {
        let mut client = InfisicalClient::new(config);
        client
            .get_secret_by_name(secret_name, folder, environment)
            .await
    });

    result
        .map(|secret| secret.value)
        .map_err(|err| FetchFailure::from(&err))
}
