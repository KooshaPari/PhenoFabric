//! Callback dispatch: state validation + auth-complete routing + HTML.
//!
//! The wire server already has the `auth_complete` token-exchange path
//! (see `wire::protocol::process_message`). This module:
//!
//! 1. Validates the CSRF `state` against the in-memory registry.
//! 2. On success, builds the JSON message the wire `auth_complete`
//!    understands and calls the existing handler. The token-exchange path
//!    therefore stays in exactly one place.
//! 3. Renders a small HTML page back to the browser so the user sees
//!    something interpretable instead of a raw JSON body.

use super::registry::{take_state_with_verifier, IssuedVerifier};
use crate::coordinator::Coordinator;

/// Response to send back to the browser.
pub(crate) struct HttpResponse {
    pub(crate) status: u16,
    pub(crate) reason: &'static str,
    pub(crate) body: String,
}

impl HttpResponse {
    pub(crate) fn html(status: u16, reason: &'static str, body: String) -> Self {
        Self {
            status,
            reason,
            body,
        }
    }
}

/// Build the JSON message the existing wire `handle_auth_complete`
/// understands and call it on the coordinator. Reuses the wire handler so
/// the token-exchange path stays in exactly one place.
///
/// `verifier` is the PKCE verifier the daemon minted for `state`. It is
/// attached to the wire message as `code_verifier` so the WorkOS token
/// exchange goes through as a public client (no `client_secret`).
///
/// If `caller_supplied_verifier` is `Some(v)`, the daemon compares `v`
/// against the minted one and rejects the callback on mismatch. This
/// preserves the older "trust a caller-supplied verifier if it matches"
/// behaviour so the wire protocol keeps working for clients that send a
/// verifier (e.g. integration tests that exercise the daemon directly).
/// A `None` means the caller did not supply one (the browser-driven
/// callback path), so the daemon uses the minted verifier directly.
pub(crate) async fn process_callback(
    coordinator: &Coordinator,
    code: Option<&str>,
    state: Option<&str>,
    caller_supplied_verifier: Option<&str>,
) -> HttpResponse {
    let Some(state) = state else {
        return HttpResponse::html(
            400,
            "Bad Request",
            render_error_page(
                "Missing state parameter",
                "The callback request did not include a `state` query parameter. \
             This request did not originate from a WorkOS authorize URL produced by \
             this daemon, so it cannot be safely processed.",
            ),
        );
    };

    let Some(code) = code else {
        return HttpResponse::html(
            400,
            "Bad Request",
            render_error_page(
                "Missing code parameter",
                "The callback request did not include a `code` query parameter. \
             Without an authorization code the daemon has nothing to exchange with \
             WorkOS.",
            ),
        );
    };

    let verifier = match take_state_with_verifier(state) {
        Some(v) => v,
        None => {
            return HttpResponse::html(
                400,
                "Bad Request",
                render_error_page(
                    "Invalid or expired state",
                    "The state parameter did not match any in-flight login this daemon \
                 issued, or it was already redeemed, or it expired. Restart the sign-in \
                 flow from the application.",
                ),
            );
        }
    };

    // Resolve which verifier to send on the exchange.
    //
    // - If the daemon minted a verifier (`IssuedVerifier::With`), the
    //   exchange MUST use it. A caller-supplied verifier is accepted
    //   only when it matches the minted one -- if the caller claimed to
    //   send a verifier but it does not match, we reject the request
    //   rather than proceeding with an unverifiable code. This is the
    //   safety net for tamper attempts. The state is re-recorded on
    //   mismatch so a legitimate user with a transport-level verifier
    //   corruption can retry the same callback URL.
    // - If the daemon did NOT mint a verifier (`IssuedVerifier::None`),
    //   the caller-supplied verifier, if any, flows through unchanged.
    //   This preserves the pre-PKCE wire-protocol contract for callers
    //   that drive the daemon directly.
    let effective_verifier: Option<String> = match (verifier, caller_supplied_verifier) {
        (IssuedVerifier::With(minted), Some(supplied)) if minted != supplied => {
            // Re-bind the verifier to the same state so the user can
            // retry. `take_state_with_verifier` already removed the
            // entry; the missing-verifier path is unreachable because
            // we just took it. A racing callback on the same state
            // would consume the re-recorded entry, but a racing callback
            // that did not have the right minted verifier would be
            // rejected by the same mismatch check.
            super::registry::record_state_with_verifier(state, Some(minted.clone()));
            return HttpResponse::html(
                400,
                "Bad Request",
                render_error_page(
                    "PKCE verifier mismatch",
                    "The callback supplied a `code_verifier` that does not match the one \
                 the daemon minted for this state. PKCE rejects the request rather \
                 than exchanging the code. Restart sign-in from the application.",
                ),
            );
        }
        (IssuedVerifier::With(minted), _) => Some(minted),
        (IssuedVerifier::None, supplied) => supplied.map(str::to_string),
    };

    // State is valid. Reuse the wire `process_message` router so the
    // token-exchange path stays in exactly one place. The verifier is
    // attached as `code_verifier`; the wire handler reads it from the
    // parsed JSON and passes it to `authenticate_with_authorization_code`.
    let message = match &effective_verifier {
        Some(v) => format!(
            r#"{{"type":"auth_complete","code":{},"code_verifier":{}}}"#,
            json_string(code),
            json_string(v)
        ),
        None => format!(r#"{{"type":"auth_complete","code":{}}}"#, json_string(code)),
    };

    let response = crate::wire::protocol::process_message(&message, coordinator).await;

    match response {
        Some(text) if looks_like_login_success(&text) => {
            HttpResponse::html(200, "OK", render_success_page())
        }
        Some(text) => {
            let err = extract_error_message(&text);
            HttpResponse::html(
                500,
                "Internal Server Error",
                render_error_page(
                    "Sign-in exchange failed",
                    &format!(
                        "The daemon received the callback but WorkOS rejected the \
                     authorization code. {err}\n\nYou can close this tab and retry \
                     sign-in from the application.",
                    ),
                ),
            )
        }
        None => HttpResponse::html(
            500,
            "Internal Server Error",
            render_error_page(
                "Sign-in exchange failed",
                "The daemon's auth-complete handler returned no response. \
             Check the daemon logs for the underlying error.",
            ),
        ),
    }
}

/// Heuristic for "the wire handler reported a successful login".
///
/// The wire `auth_complete` success path emits `"type":"auth_complete_response"`
/// with `"logged_in":true` (the cross-crate contract pinned by the
/// `auth_complete_success_parses_as_gui_auth_status` test). A bare
/// substring check is enough here: the response is generated by code we
/// control and any failure path produces an `auth_error` instead.
fn looks_like_login_success(text: &str) -> bool {
    text.contains("\"logged_in\":true") || text.contains("\"logged_in\": true")
}

fn json_string(s: &str) -> String {
    serde_json::Value::String(s.to_string()).to_string()
}

/// Extract a human-readable error message from a wire-protocol JSON
/// response produced by `auth_complete`'s failure paths.
///
/// Best effort: if the response isn't well-formed JSON, return the raw
/// text. The HTML page wraps this in a sentence so the user sees
/// something interpretable either way.
fn extract_error_message(text: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(message) = value.get("message").and_then(|v| v.as_str()) {
            return message.to_string();
        }
        if let Some(error) = value.get("error").and_then(|v| v.as_str()) {
            return error.to_string();
        }
    }
    text.to_string()
}

fn render_success_page() -> String {
    "<!doctype html>\n\
     <html lang=\"en\">\n\
     <head><meta charset=\"utf-8\"><title>Signed in</title></head>\n\
     <body>\n\
     <h1>Signed in</h1>\n\
     <p>You may close this tab and return to the application.</p>\n\
     </body>\n\
     </html>\n"
        .to_string()
}

pub(super) fn render_error_page(title: &str, body: &str) -> String {
    let safe_title = html_escape(title);
    let safe_body = html_escape(body);
    format!(
        "<!doctype html>\n\
         <html lang=\"en\">\n\
         <head><meta charset=\"utf-8\"><title>{safe_title}</title></head>\n\
         <body>\n\
         <h1>{safe_title}</h1>\n\
         <p>{safe_body}</p>\n\
         </body>\n\
         </html>\n"
    )
}

pub(super) fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
