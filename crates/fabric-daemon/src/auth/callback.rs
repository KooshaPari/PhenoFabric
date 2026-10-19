//! OAuth/HTTP callback listener for the WorkOS AuthKit redirect URI.
//!
//! The WorkOS authorize URL embeds `state=<random>` and `redirect_uri=<this
//! listener>`. After the user signs in, WorkOS redirects the browser back
//! to that URI with `?code=...&state=...`. The wire server speaks
//! newline-delimited JSON and cannot serve HTTP, so without this module
//! the registered `workos_redirect_uri` would be a dead endpoint and the
//! browser would just hang on a blank page.
//!
//! Design choices and their reasons:
//!
//! * **Hand-rolled HTTP parsing** on `std::net::TcpListener` rather than
//!   pulling in an HTTP server crate: a single GET with a query string is
//!   small enough that the cost of `axum`/`hyper`/`tiny_http` (binary
//!   size, dep graph churn, CVE surface) is not worth it. The wire
//!   server already uses the same `std::net::TcpListener` shape.
//!
//! * **In-memory state registry**, not a daemon-wide field on the
//!   coordinator. The state is generated inside `WorkOsProvider` and the
//!   provider is recreated per request (see
//!   `coordinator::workos_provider`), so the only reliable hook point is
//!   the wire-side `handle_auth_start` itself, which inserts into this
//!   registry. The callback then validates-and-consumes from the same
//!   registry. One-shot semantics: a state can be redeemed exactly once,
//!   which is what RFC 6749 section 10.12 calls for and what makes the
//!   CSRF check meaningful.
//!
//! * **TTL window of 10 minutes** (TTL constant in `registry`). Long
//!   enough for the user to read an email and click a Magic Auth link,
//!   short enough that a leaked state cannot be replayed after a session
//!   break.
//!
//! * **Bind failure does not panic the daemon.** If the configured
//!   `callback_listen_addr` is unavailable (e.g. port already in use on a
//!   host that also runs another service), the daemon logs the error and
//!   continues serving the wire server. OAuth login is broken in that
//!   state, but the daemon's primary job (transport) keeps running.
//!
//! Module split (all under the 500-line hard limit):
//!
//! | Submodule | Concern | Public items |
//! |-----------|---------|--------------|
//! | `registry` | State storage, TTL, one-shot semantics | `record_state`, `take_state`, `STATE_TTL` (all `pub(crate)`) |
//! | `server`   | HTTP listener loop, request parsing, response writing | `run_callback_server` |
//! | `dispatch` | CSRF/state validation + auth-complete routing + HTML rendering | `process_callback` (used by `server`) |
//! | `tests`    | 13 tests on real loopback sockets | n/a |

mod dispatch;
mod registry;
mod server;

#[cfg(test)]
mod tests;

// `record_state` and `record_state_with_verifier` are the two items the
// wire server needs to call from `auth_handlers::handle_auth_start`.
// Tests and sibling submodules inside `auth::callback` import the rest
// directly from `super::registry` or `super::dispatch`, so no further
// re-exports are needed.
#[allow(unused_imports)] // `record_state` is re-exported for the public API.
pub use registry::{record_state, record_state_with_verifier};
pub use server::run_callback_server;
